use core::future::Future;

use crate::{processor::ExecutionError, ShortID, Stack};

#[cfg(feature = "alloc")]
mod dynamic;

#[cfg(feature = "alloc")]
pub use dynamic::DynamicExecutionQueue;

pub trait ExecutionQueue {
    fn run(
        &mut self,
        start_id: Option<ShortID>,
        stack: &mut dyn Stack,
    ) -> Result<(), ExecutionError>;
}

#[cfg(feature = "nightly")]
pub trait AsyncExecutionQueue {
    /// Cumulative stack usage of all contained processors.
    /// Read [`EmbeddedProcessor::STACK_USAGE`](crate::processor::EmbeddedProcessor::STACK_USAGE) for more details.
    const STACK_USAGE: usize;

    type Fut<'s>: Future<Output = Result<(), ExecutionError>> + 's
    where
        Self: 's;

    fn run<'s>(&'s mut self, start_id: Option<ShortID>, stack: &'s mut dyn Stack) -> Self::Fut<'s>;
}
