use super::engine::Command;
use crate::constants::AVG_CMD_COUNT;
use smallvec::SmallVec;

mod ext;
pub use ext::*;

// TODO Make this mod private once all the compilation stuff has been moved out!
pub mod binary;
pub use binary::BinaryDictionary;

pub type CommandList<OutputCommand> = SmallVec<[Command<OutputCommand>; AVG_CMD_COUNT]>;

pub trait Dictionary {
    type Stroke;
    type OutputCommand;

    fn lookup(&mut self, outline: &[Self::Stroke]) -> Option<CommandList<Self::OutputCommand>>;
    fn fallback_commands(&self, stroke: &Self::Stroke) -> CommandList<Self::OutputCommand>;

    fn longest_outline_length(&self) -> usize;
}
