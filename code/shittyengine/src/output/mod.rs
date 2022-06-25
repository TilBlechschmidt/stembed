mod command;
pub use command::OutputCommand;

#[cfg(feature = "alloc")]
mod aggregator;
#[cfg(feature = "alloc")]
pub use aggregator::OutputAggregator;

#[cfg(feature = "desktop")]
mod os;
#[cfg(feature = "desktop")]
pub use os::OSOutput;

pub trait OutputProcessor {
    fn apply<I: Iterator<Item = char>>(&mut self, command: OutputCommand<I>);
}
