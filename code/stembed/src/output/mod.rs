use crate::core::processor::OutputInstructionSet;

#[cfg(feature = "desktop")]
mod os;
#[cfg(feature = "desktop")]
pub use os::*;

pub trait OutputSink {
    type Instruction;

    fn send(&mut self, output: OutputInstructionSet<Self::Instruction>);
}
