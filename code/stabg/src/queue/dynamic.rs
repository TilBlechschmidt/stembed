use super::ExecutionQueue;
use crate::{
    context::ExecutionContext,
    identifier::ShortID,
    processor::Processor,
    processor::{ExecutionError, InitializationContext, InitializationError},
    registry::DynamicRegistry,
    stack::Stack,
    Identifier,
};
use alloc::{boxed::Box, vec::Vec};
use core::ops::{Deref, DerefMut};
use log::{error, warn};

type OwnedProcessor = Box<dyn Processor>;

struct LoadedProcessor {
    processor: OwnedProcessor,
    input: Vec<Identifier>,
    output: Vec<Identifier>,
}

/// Self-organizing [`ExecutionQueue`](ExecutionQueue) on the heap
#[doc(cfg(feature = "alloc"))]
pub struct DynamicExecutionQueue {
    processors: Vec<LoadedProcessor>,
    registry: DynamicRegistry,
    abort_on_error: bool,
}

impl DynamicExecutionQueue {
    // TODO Implement extend or generally anything that allows us to go over iterators :)

    /// Creates an empty queue
    pub fn new() -> Self {
        Self {
            processors: Vec::new(),
            registry: DynamicRegistry::new(),
            abort_on_error: false,
        }
    }

    /// Sets whether execution should be aborted when any single processor in the collection fails
    pub fn abort_on_error(&mut self, enabled: bool) -> &mut Self {
        self.abort_on_error = enabled;
        self
    }

    /// Adds the given processor to the collection, calling its [`load`](Processor::load) method to determine input & output dependencies
    pub fn schedule<P: Processor + 'static>(
        &mut self,
        processor: P,
    ) -> Result<&mut Self, InitializationError> {
        self.schedule_boxed(Box::new(processor))
    }

    // TODO Output a list of diagnostics for further processing / display / testing :)
    /// Reorders the processors as required, checks for unmet and cyclic dependencies, emits warnings for unused outputs.
    pub fn optimize(&mut self) {
        self.reorder_processors();
        self.lint_unused_outputs();
    }

    /// Removes a processor from scheduling and calls its [`unload`](Processor::unload) method.
    ///
    /// After unloading a processor, the execution order may be suboptimal and calling [`optimize`](Self::optimize) is recommended.
    pub fn unload(&mut self, identifier: Identifier) {
        // Remove the processor, unloading is called by the `Drop` impl
        self.processors
            .retain(|processor| processor.identifier() != identifier);

        // Re-register all types to get rid of potentially unused types
        self.registry = DynamicRegistry::new();
        self.processors
            .iter()
            .flat_map(|p| p.input.iter().chain(p.output.iter()))
            .for_each(|t| {
                self.registry.register(t);
            });
    }

    fn schedule_boxed(
        &mut self,
        mut processor: OwnedProcessor,
    ) -> Result<&mut Self, InitializationError> {
        let mut ctx = InitializationContext::new();
        processor.load(&mut ctx)?;

        ctx.input.iter().chain(ctx.output.iter()).for_each(|t| {
            self.registry.register(t);
        });

        self.processors.push(LoadedProcessor::new(processor, ctx));

        Ok(self)
    }

    fn reorder_processors(&mut self) {
        let mut available_types = Vec::<Identifier>::new();
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
                    .map(|i| *i)
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
    }

    fn lint_unused_outputs(&self) {
        let mut type_stack = Vec::<(Identifier, Identifier, bool)>::new();

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

        for (processor_ident, type_ident, _) in type_stack.iter().filter(|(_, _, used)| !used) {
            warn!(
                "Output of type '{}' from processor '{}' is unused",
                type_ident, processor_ident
            );
        }
    }
}

impl ExecutionQueue for DynamicExecutionQueue {
    fn run(
        &mut self,
        start_id: Option<ShortID>,
        stack: &mut dyn Stack,
    ) -> Result<(), ExecutionError> {
        // 1. Annotate processors with increasing IDs
        // 2. Skip anything before the start_id if applicable
        let pending_processors = self
            .processors
            .iter_mut()
            .enumerate()
            .map(|(i, p)| (i as u32, p))
            .skip_while(|(i, _)| {
                if let Some(start) = start_id {
                    start == *i
                } else {
                    false
                }
            });

        // Go through all processors
        for (id, processor) in pending_processors {
            let context = ExecutionContext::new(stack, id, &mut self.registry);
            let result = processor.process(context);

            if self.abort_on_error {
                result?;
            } else if let Err(e) = result {
                error!(
                    "Failed to execute processor {}: {:?}",
                    processor.identifier(),
                    e
                );
            }
        }

        Ok(())
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
