use crate::{
    context::ExecutionContext,
    identifier::ShortID,
    processor::{InitializationContext, InitializationError, OwnedProcessor},
    registry::TypeRegistry,
    stack::Stack,
    Identifier,
};
use alloc::vec::Vec;
use core::ops::{Deref, DerefMut};
use log::{error, warn};

struct LoadedProcessor {
    processor: OwnedProcessor,
    input: Vec<ShortID>,
    output: Vec<ShortID>,
}

pub struct ProcessorCollection<'r> {
    processors: Vec<LoadedProcessor>,
    type_registry: &'r mut TypeRegistry,
}

impl<'r> ProcessorCollection<'r> {
    pub fn new(type_registry: &'r mut TypeRegistry) -> Self {
        Self {
            processors: Vec::new(),
            type_registry,
        }
    }

    // TODO Implement extend or generally anything that allows us to go over iterators :)
    // TODO Make a generic version which accepts any processor and internally boxes it
    pub fn push(&mut self, mut processor: OwnedProcessor) -> Result<(), InitializationError> {
        let mut ctx = InitializationContext::new(self.type_registry);
        processor.load(&mut ctx)?;

        self.processors.push(LoadedProcessor::new(processor, ctx));

        Ok(())
    }

    fn optimize_execution_order(&mut self) -> bool {
        let mut available_types = Vec::<ShortID>::new();
        let mut processor_stack = Vec::with_capacity(self.processors.len());
        let mut success = true;

        while !self.processors.is_empty() {
            let resolved_processors =
                drain_filter(&mut self.processors, |p| is_sub(&available_types, &p.input));

            if resolved_processors.is_empty() {
                success = false;
                break;
            }

            for processor in resolved_processors.iter() {
                available_types.extend(processor.output.iter());
            }

            processor_stack.extend(resolved_processors.into_iter());
        }

        if !success {
            // TODO Do a full dependency graph analysis to figure out cyclic dependencies
            //      and actual unmet inputs over transitive unmet inputs.
            for processor in self.processors.iter() {
                let unmet_types = processor
                    .input
                    .iter()
                    .filter(|i| !available_types.contains(i))
                    .filter_map(|i| self.type_registry.reverse_lookup(*i))
                    .collect::<Vec<_>>()
                    .join(", ");

                warn!(
                    "Unable to satisfy input dependencies of {}: {}",
                    processor.identifier(),
                    unmet_types
                );
            }

            // Push the processors to the end of the stack so that they have the best chance of doing *something*
            processor_stack.append(&mut self.processors);
        }

        // Move the processors back into the original collection
        self.processors.extend(processor_stack.into_iter());

        success
    }

    fn lint_unused_outputs(&self) {
        let mut type_stack = Vec::<(Identifier, ShortID, bool)>::new();

        for processor in self.processors.iter() {
            let i = processor.identifier();

            type_stack
                .iter_mut()
                .filter(|(_, t, _)| processor.input.contains(t))
                .for_each(|(_, _, used)| {
                    *used = true;
                });

            type_stack.extend(processor.output.iter().map(|t| (i, *t, false)));
        }

        for (processor_ident, t, _) in type_stack.iter().filter(|(_, _, used)| !used) {
            warn!(
                "Output of type '{}' from processor '{}' is unused",
                self.type_registry.reverse_lookup(*t).unwrap(),
                processor_ident
            );
        }
    }

    // TODO Output a list of diagnostics for further processing / display / testing :)
    pub fn build(mut self) -> impl FnMut(Option<ShortID>, &mut dyn Stack, &TypeRegistry) {
        if !self.optimize_execution_order() {
            error!("Failed to optimize execution order, some processors may fail to execute due to missing inputs");
        }

        self.lint_unused_outputs();

        move |start_id, stack, registry| {
            // 1. Annotate processors with increasing IDs
            // 2. Skip anything before the start_id if applicable
            let pending_processors = self
                .processors
                .iter_mut()
                .enumerate()
                .map(|(i, p)| (i as u8, p))
                .skip_while(|(i, _)| {
                    if let Some(start) = start_id {
                        *start == *i as u8
                    } else {
                        false
                    }
                });

            // Go through all processors
            for (id, processor) in pending_processors {
                let context = ExecutionContext::new(stack, id.into(), registry);
                processor.process(context);
            }
        }
    }
}

impl LoadedProcessor {
    fn new(processor: OwnedProcessor, ctx: InitializationContext) -> Self {
        Self {
            processor,
            input: ctx.input,
            output: ctx.output,
        }
    }
}

impl Deref for LoadedProcessor {
    type Target = OwnedProcessor;

    fn deref(&self) -> &Self::Target {
        &self.processor
    }
}

impl DerefMut for LoadedProcessor {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.processor
    }
}

impl Drop for LoadedProcessor {
    fn drop(&mut self) {
        self.processor.unload();
    }
}

fn is_sub<T: PartialEq>(mut haystack: &[T], needle: &[T]) -> bool {
    if needle.is_empty() {
        return true;
    }

    while !haystack.is_empty() {
        if haystack.starts_with(needle) {
            return true;
        }
        haystack = &haystack[1..];
    }

    false
}

fn drain_filter<T, F>(source: &mut Vec<T>, mut predicate: F) -> Vec<T>
where
    F: FnMut(&T) -> bool,
{
    let mut taken = Vec::new();

    for i in 0..source.len() {
        let index = i - taken.len();
        if predicate(&source[index]) {
            taken.push(source.remove(index));
        }
    }

    taken
}

#[cfg(test)]
mod does {
    use alloc::vec;

    use super::drain_filter;

    #[test]
    fn drain_correctly() {
        let source = vec![0, 1, 2, 3, 4];

        let mut data = source.clone();
        let drained = drain_filter(&mut data, |_| true);
        assert_eq!(drained, [0, 1, 2, 3, 4]);
        assert_eq!(data, []);

        let mut data = source.clone();
        let drained = drain_filter(&mut data, |_| false);
        assert_eq!(drained, []);
        assert_eq!(data, [0, 1, 2, 3, 4]);

        let mut data = source.clone();
        let drained = drain_filter(&mut data, |i| *i == 2);
        assert_eq!(drained, [2]);
        assert_eq!(data, [0, 1, 3, 4]);

        let mut data = source.clone();
        let drained = drain_filter(&mut data, |i| *i > 0 && *i < 4);
        assert_eq!(drained, [1, 2, 3]);
        assert_eq!(data, [0, 4]);
    }
}
