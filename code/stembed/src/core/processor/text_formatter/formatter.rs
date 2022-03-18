use super::super::{CommandProcessor, OutputInstructionSet};
use super::{TextFormatterState, TextOutputCommand, TextOutputInstruction};
use crate::constants::{AVG_OUTPUT_INSTRUCTIONS, HISTORY_SIZE};
use crate::core::engine::{CommandDelta, HistoryBuffer};

const COMMAND_HISTORY_SIZE: usize = HISTORY_SIZE * AVG_OUTPUT_INSTRUCTIONS;

struct UndoInfo {
    character_count: usize,
}

impl UndoInfo {
    const EMPTY: Self = UndoInfo { character_count: 0 };
}

pub struct TextFormatter {
    // TODO Use the same alloc-less history buffer data type as in the Engine
    history: HistoryBuffer<(TextFormatterState, UndoInfo), COMMAND_HISTORY_SIZE>,
}

impl TextFormatter {
    pub fn new() -> Self {
        Self {
            history: HistoryBuffer::new(),
        }
    }

    fn undo(&mut self) -> Option<TextOutputInstruction> {
        self.history
            .pop()
            .map(|(_, undo_info)| TextOutputInstruction::Backspace(undo_info.character_count))
    }

    fn apply(&mut self, command: TextOutputCommand) -> Option<TextOutputInstruction> {
        use TextOutputCommand::*;
        let mut state = self.state();

        let (undo_info, output) = match command {
            Write(mut string) => {
                // 1. Mutate string according to current state
                string = state.apply(string);

                // 2. Advance state
                state.tick();

                // 3. Output result
                (
                    UndoInfo {
                        character_count: string.len(),
                    },
                    Some(TextOutputInstruction::Write(string)),
                )
            }
            ChangeCapitalization(capitalization) => {
                state.capitalization.change_to(capitalization);
                (UndoInfo::EMPTY, None)
            }
            ChangeAttachment(attachment) => {
                state.attachment.change_to(attachment);
                (UndoInfo::EMPTY, None)
            }
            ChangeDelimiter(delimiter) => {
                state.delimiter = delimiter;
                (UndoInfo::EMPTY, None)
            }
            ResetFormatting => {
                state = TextFormatterState::default();
                (UndoInfo::EMPTY, None)
            }
        };

        self.history.push((state, undo_info));

        output
    }

    fn state(&self) -> TextFormatterState {
        self.history
            .back()
            .map(|(s, _)| s)
            .cloned()
            .unwrap_or_default()
    }
}

impl CommandProcessor for TextFormatter {
    type OutputCommand = TextOutputCommand;
    type OutputInstruction = TextOutputInstruction;

    fn consume(
        &mut self,
        delta: CommandDelta<Self::OutputCommand>,
    ) -> OutputInstructionSet<Self::OutputInstruction> {
        enum CommandType {
            Undo,
            Apply(TextOutputCommand),
        }

        (0..delta.to_undo)
            .map(|_| CommandType::Undo)
            .chain(delta.to_push.into_iter().map(|c| CommandType::Apply(c)))
            .filter_map(|command_type| match command_type {
                CommandType::Undo => self.undo(),
                CommandType::Apply(command) => self.apply(command),
            })
            .collect()
    }
}
