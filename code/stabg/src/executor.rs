use crate::{
    context::{ProcessorBoundary, ValueSet},
    processor::ExecutionError,
    registry::Registry,
    stack::Stack,
    Identifiable, ShortID,
};
use core::future::Future;

pub struct Executor<'s, 'r> {
    stack: &'s mut dyn Stack,
    type_registry: &'r mut dyn Registry,
}

impl<'s, 'r> Executor<'s, 'r> {
    pub fn new(stack: &'s mut dyn Stack, type_registry: &'r mut dyn Registry) -> Self {
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
            if id == id_procbound {
                debug_assert!(
                    active_valueset_count.is_none(),
                    "encountered ProcessorBoundary inside ValueSet"
                );
                previous_processor = current_processor;
                current_processor = Some(ShortID::from_be_bytes([
                    value[0], value[1], value[2], value[3],
                ]));
            } else if id == id_valueset {
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
                    .push(id_valueset, &(count - 1).to_be_bytes())
                    .expect("failed to recreate ValueSet");
                self.stack
                    .push(id_procbound, &(current_processor.unwrap()).to_be_bytes())
                    .expect("failed to recreate ValueSet proc marker");
                return Some(previous_proc);
            }
        }

        None
    }

    pub fn execute_sync<F>(&mut self, execution_queue: &mut F) -> Result<(), ExecutionError>
    where
        F: FnMut(Option<ShortID>, &mut dyn Stack, &dyn Registry) -> Result<(), ExecutionError>,
    {
        // Remove any remainders from previous runs
        self.stack.clear();

        // Run through it once completely
        (execution_queue)(None, self.stack, self.type_registry)?;

        // Repeat until there are no branches left
        while let Some(start_point) = self.next_execution_step() {
            (execution_queue)(Some(start_point), self.stack, self.type_registry)?;
        }

        Ok(())
    }

    // pub async fn execute_async<'a, 's1, 'r1, F, Fut>(
    //     &'a mut self,
    //     execution_queue: &mut F,
    // ) -> Result<(), ExecutionError>
    // where
    //     Fut: Future<Output = Result<(), ExecutionError>> + 's1 + 'r1,
    //     F: FnMut(Option<ShortID>, &'s1 mut (dyn Stack + 's1), &'r1 (dyn Registry + 'r1)) -> Fut,
    //     's: 's1,
    //     'r: 'r1,
    //     'r: 's1,
    //     's: 'r1,
    //     'a: 's1,
    //     'a: 'r1,
    // {
    //     // Remove any remainders from previous runs
    //     self.stack.clear();

    //     // Run through it once completely
    //     (execution_queue)(None, self.stack, self.type_registry).await?;

    //     // Repeat until there are no branches left
    //     // while let Some(start_point) = self.next_execution_step() {
    //     //     (execution_queue)(Some(start_point), self.stack, self.type_registry).await?;
    //     // }

    //     Ok(())
    // }

    // pub async fn execute_async<F, Fut>(&mut self, execution_queue: F) -> Result<(), ExecutionError>
    // where
    //     // Fut: Future<Output = Result<(), ExecutionError>>,
    //     // F: FnMut(Option<ShortID>, &mut (dyn Stack), &(dyn Registry)) -> Fut,
    //     for<'anys, 'anyr> F: FnOnce<(
    //         Option<ShortID>,
    //         &'anys mut (dyn Stack + 'anys),
    //         &'anyr (dyn Registry + 'anyr),
    //     )>,
    //     for<'anys, 'anyr> <F as FnOnce<(
    //         Option<ShortID>,
    //         &'anys mut (dyn Stack + 'anys),
    //         &'anyr (dyn Registry + 'anyr),
    //     )>>::Output: Future<Output = Result<(), ExecutionError>> + 'anys + 'anyr,
    // {
    //     // Remove any remainders from previous runs
    //     self.stack.clear();

    //     // Run through it once completely
    //     (execution_queue)(None, self.stack, self.type_registry).await?;

    //     // Repeat until there are no branches left
    //     // while let Some(start_point) = self.next_execution_step() {
    //     //     (execution_queue)(Some(start_point), self.stack, self.type_registry).await?;
    //     // }

    //     Ok(())
    // }

    pub async fn execute_async<'a, 'b, 'c>(
        &'c mut self,
        execution_queue: impl AsyncExecutionQueue<'a, 'b>,
    ) -> Result<(), ExecutionError>
    where
        'c: 'a,
        'r: 'a,
        's: 'a,
        'c: 'b,
        'r: 'b,
        's: 'b,
    {
        // Remove any remainders from previous runs
        self.stack.clear();

        // Run through it once completely
        execution_queue
            .call(None, self.stack, self.type_registry)
            .await?;

        // Repeat until there are no branches left
        // while let Some(start_point) = self.next_execution_step() {
        //     execution_queue.call(Some(start_point), self.stack, self.type_registry).await?;
        // }

        Ok(())
    }
}

// Idea blatantly stolen from:
// https://github.com/rust-lang/wg-async/blob/master/src/vision/submitted_stories/status_quo/alan_writes_a_web_framework.md

pub trait AsyncExecutionQueue<'s, 'r> {
    type Fut: core::future::Future<Output = Result<(), ExecutionError>> + 's + 'r;
    fn call(
        self,
        start_id: Option<ShortID>,
        stack: &'s mut dyn Stack,
        registry: &'r dyn Registry,
    ) -> Self::Fut;
}

impl<'s, 'r, Fut, F> AsyncExecutionQueue<'s, 'r> for F
where
    F: FnOnce(Option<ShortID>, &'s mut dyn Stack, &'r dyn Registry) -> Fut,
    Fut: core::future::Future<Output = Result<(), ExecutionError>> + 's + 'r,
{
    type Fut = Fut;
    fn call(
        self,
        start_id: Option<ShortID>,
        stack: &'s mut dyn Stack,
        registry: &'r dyn Registry,
    ) -> Fut {
        self(start_id, stack, registry)
    }
}
