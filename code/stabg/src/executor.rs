use crate::{
    context::{ProcessorBoundary, ValueSet},
    registry::TypeRegistry,
    stack::Stack,
    Identifiable, ShortID,
};
use core::future::Future;

pub struct Executor<'s, 'r> {
    stack: &'s mut dyn Stack,
    type_registry: &'r mut TypeRegistry,
}

impl<'s, 'r> Executor<'s, 'r> {
    pub fn new(stack: &'s mut dyn Stack, type_registry: &'r mut TypeRegistry) -> Self {
        Self {
            stack,
            type_registry,
        }
    }

    fn next_execution_step(&mut self) -> Option<ShortID> {
        // TODO These should really always be present or an error may be thrown!
        let id_valueset = self.type_registry.lookup(ValueSet::IDENTIFIER)?;
        let id_procbound = self.type_registry.lookup(ProcessorBoundary::IDENTIFIER)?;

        let mut previous_processor: Option<ShortID> = None;
        let mut current_processor: Option<ShortID> = None;
        let mut active_valueset_count: Option<u8> = None;

        // Removes values from the stack while:
        // - Tracking the active and previously active processor
        // - Tracking whether we are inside a ValueSet and the number of remaining items
        // Based on that it:
        // - Discards values outside of ValueSets without exception
        // - Discards ValueSets which only contain one value
        // - Stops when encountering a ValueSet with more than one value
        //     -> Only if a processor has been executed after it (i.e. current & previous proc are Some)
        //     - Consumes the old ValueSet header
        //     - Consumes the latest value
        //     - Pushes back a new ValueSet header and the previously removed processor boundary
        //     - Returns the previous processor (the first one after the one that created the ValueSet)
        // - Stops when it reaches the bottom of the stack
        while let Some((id, value)) = self.stack.pop() {
            if id == id_procbound {
                debug_assert!(
                    active_valueset_count.is_none(),
                    "encountered ProcessorBoundary inside ValueSet"
                );
                previous_processor = current_processor;
                current_processor = Some(value[0].into());
            } else if id == id_valueset {
                debug_assert!(
                    active_valueset_count.is_none(),
                    "encountered ValueSet inside ValueSet"
                );
                assert!(
                    previous_processor.or(current_processor).is_some(),
                    "encountered ValueSet without active processor"
                );
                active_valueset_count = Some(value[0]);
            } else if let Some(1) = active_valueset_count {
                active_valueset_count = None;
            } else if let (Some(count), Some(previous_proc)) =
                (active_valueset_count, previous_processor)
            {
                // These should theoretically always succeed because we popped the values just prior,
                // thus enough space should be available. However, since it is blackbox magic,
                // we use expect here to play nice :)
                self.stack
                    .push(id_valueset, &[count - 1])
                    .expect("failed to recreate ValueSet");
                self.stack
                    .push(id_procbound, &[*current_processor.unwrap()])
                    .expect("failed to recreate ValueSet proc marker");
                return Some(previous_proc);
            }
        }

        None
    }

    pub fn execute_sync<F>(&mut self, execution_queue: &mut F)
    where
        F: FnMut(Option<ShortID>, &mut dyn Stack, &TypeRegistry),
    {
        // Remove any remainders from previous runs
        self.stack.clear();

        // Run through it once completely
        (execution_queue)(None, self.stack, self.type_registry);

        // Repeat until there are no branches left
        while let Some(start_point) = self.next_execution_step() {
            (execution_queue)(Some(start_point), self.stack, self.type_registry);
        }
    }

    pub async fn execute_async<F, Fut>(&mut self, execution_queue: &mut F)
    where
        Fut: Future<Output = ()>,
        F: FnMut(Option<ShortID>, &mut dyn Stack, &TypeRegistry) -> Fut,
    {
        // Remove any remainders from previous runs
        self.stack.clear();

        // Run through it once completely
        (execution_queue)(None, self.stack, self.type_registry);

        // Repeat until there are no branches left
        while let Some(start_point) = self.next_execution_step() {
            (execution_queue)(Some(start_point), self.stack, self.type_registry).await;
        }
    }
}
