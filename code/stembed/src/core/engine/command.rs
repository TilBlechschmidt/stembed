use crate::constants::AVG_CMD_COUNT;
use smallvec::SmallVec;

/// Instructions processed by the engine itself
#[derive(Clone, Copy, Debug)]
pub enum EngineCommand {
    /// Removes the previous outline from the stack
    UndoPrevious,
}

/// Instruction for either the engine or the output processor
#[derive(Clone, Copy, Debug)]
pub enum Command<OutputCommand> {
    /// Variant which will be forwarded to the output processor
    Output(OutputCommand),
    /// Variant which will be processed by the engine itself
    Engine(EngineCommand),
}

/// Command delta calculated by the engine
#[derive(Debug, PartialEq, Eq)]
pub struct CommandDelta<OutputCommand> {
    pub to_undo: usize,
    pub to_push: SmallVec<[OutputCommand; AVG_CMD_COUNT]>,
}

impl<OutputCommand> CommandDelta<OutputCommand> {
    pub fn assimilate(&mut self, other: Self) {
        self.to_undo += other.to_undo;
        self.to_push.extend(other.to_push);
    }
}

impl<OutputCommand> Default for CommandDelta<OutputCommand> {
    fn default() -> Self {
        Self {
            to_undo: Default::default(),
            to_push: Default::default(),
        }
    }
}
