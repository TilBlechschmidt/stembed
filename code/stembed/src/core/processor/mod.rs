use super::engine::CommandDelta;
use crate::constants::AVG_OUTPUT_INSTRUCTIONS;
use smallvec::SmallVec;

pub mod text_formatter;

pub type OutputInstructionSet<OutputInstruction> =
    SmallVec<[OutputInstruction; AVG_OUTPUT_INSTRUCTIONS]>;

pub trait CommandProcessor {
    type OutputCommand;
    type OutputInstruction;

    fn consume(
        &mut self,
        delta: CommandDelta<Self::OutputCommand>,
    ) -> OutputInstructionSet<Self::OutputInstruction>;
}
