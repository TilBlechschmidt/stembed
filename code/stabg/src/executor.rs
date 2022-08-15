use crate::{
    registry::{ID_PROC_MARK, ID_VALUE_SET},
    stack::Stack,
    ShortID,
};

pub struct Executor<'s> {
    stack: &'s mut dyn Stack,
}

impl<'s> Executor<'s> {
    pub fn new(stack: &'s mut dyn Stack) -> Self {
        Self { stack }
    }

    fn next_execution_step(&mut self) -> Option<ShortID> {
        let mut previous_processor: Option<ShortID> = None;
        let mut current_processor: Option<ShortID> = None;
        let mut active_valueset_count: Option<u32> = None;

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
            if id == ID_PROC_MARK {
                debug_assert!(
                    active_valueset_count.is_none(),
                    "encountered ProcessorBoundary inside ValueSet"
                );
                previous_processor = current_processor;
                current_processor = Some(ShortID::from_be_bytes([
                    value[0], value[1], value[2], value[3],
                ]));
            } else if id == ID_VALUE_SET {
                debug_assert!(
                    active_valueset_count.is_none(),
                    "encountered ValueSet inside ValueSet"
                );
                assert!(
                    previous_processor.or(current_processor).is_some(),
                    "encountered ValueSet without active processor"
                );
                active_valueset_count =
                    Some(u32::from_be_bytes([value[0], value[1], value[2], value[3]]));
            } else if let Some(1) = active_valueset_count {
                active_valueset_count = None;
            } else if let (Some(count), Some(previous_proc)) =
                (active_valueset_count, previous_processor)
            {
                // These should theoretically always succeed because we popped the values just prior,
                // thus enough space should be available. However, since it is blackbox magic,
                // we use expect here to play nice :)
                self.stack
                    .push(ID_VALUE_SET, &(count - 1).to_be_bytes())
                    .expect("failed to recreate ValueSet");
                self.stack
                    .push(ID_PROC_MARK, &(current_processor.unwrap()).to_be_bytes())
                    .expect("failed to recreate ValueSet proc marker");
                return Some(previous_proc);
            }
        }

        None
    }

    #[cfg(feature = "alloc")]
    pub fn execute_sync(
        &mut self,
        execution_queue: &mut dyn crate::ExecutionQueue,
    ) -> Result<(), crate::processor::ExecutionError> {
        // Remove any remainders from previous runs
        self.stack.clear();

        // Run through it once completely
        execution_queue.run(None, self.stack)?;

        // Repeat until there are no branches left
        while let Some(start_point) = self.next_execution_step() {
            execution_queue.run(Some(start_point), self.stack)?;
        }

        Ok(())
    }

    #[cfg(feature = "nightly")]
    pub async fn execute_async<Q: crate::AsyncExecutionQueue>(
        &mut self,
        execution_queue: &mut Q,
    ) -> Result<(), crate::processor::EmbeddedExecutionError> {
        // Remove any remainders from previous runs
        self.stack.clear();

        // Run through it once completely
        execution_queue.run(None, self.stack).await?;

        // Repeat until there are no branches left
        while let Some(start_point) = self.next_execution_step() {
            execution_queue.run(Some(start_point), self.stack).await?;
        }

        Ok(())
    }
}
