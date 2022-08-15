#[cfg(feature = "alloc")]
mod dynamic;

#[cfg(feature = "alloc")]
pub use dynamic::DynamicExecutionQueue;

#[cfg(feature = "alloc")]
pub trait ExecutionQueue {
    fn run(
        &mut self,
        start_id: Option<crate::ShortID>,
        stack: &mut dyn crate::Stack,
    ) -> Result<(), crate::processor::ExecutionError>;
}

#[cfg(feature = "nightly")]
pub trait AsyncExecutionQueue {
    /// Cumulative stack usage of all contained processors.
    /// Read [`EmbeddedProcessor::STACK_USAGE`](crate::processor::EmbeddedProcessor::STACK_USAGE) for more details.
    const STACK_USAGE: usize;

    /// Number of processors in the queue, used for calculating per-processor stack overhead
    const PROCESSOR_COUNT: usize;

    type Fut<'s>: core::future::Future<Output = Result<(), crate::processor::EmbeddedExecutionError>>
        + 's
    where
        Self: 's;

    fn run<'s>(
        &'s mut self,
        start_id: Option<crate::ShortID>,
        stack: &'s mut dyn crate::Stack,
    ) -> Self::Fut<'s>;
}
