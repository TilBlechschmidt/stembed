use crate::core::processor::OutputInstructionSet;

mod os;
pub use os::*;

pub trait OutputSink {
    type Instruction;

    fn send(&mut self, output: OutputInstructionSet<Self::Instruction>);
}
