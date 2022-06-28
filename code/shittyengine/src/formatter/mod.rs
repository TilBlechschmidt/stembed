use crate::buffer::HistoryBuffer;
use crate::output::OutputCommand;
use crate::ORTHOGRAPHIC_SUFFIX_LENGTH;
use arrayvec::ArrayString;

mod command;
mod state;

use self::state::TextFormatterState;
pub use command::*;

type EmptyCharIter = core::iter::Empty<char>;

struct UndoInfo {
    character_count: u8,
    trailing_suffix: Option<u8>,
}

impl UndoInfo {
    const EMPTY: Self = Self {
        character_count: 0,
        trailing_suffix: None,
    };
}

pub struct Formatter<const HISTORY_SIZE: usize> {
    history: HistoryBuffer<(TextFormatterState, UndoInfo), HISTORY_SIZE>,
    latest_suffix: Option<ArrayString<ORTHOGRAPHIC_SUFFIX_LENGTH>>,
}

impl<const HISTORY_SIZE: usize> Formatter<HISTORY_SIZE> {
    pub fn new() -> Self {
        Self {
            history: HistoryBuffer::new(),
            latest_suffix: None,
        }
    }

    fn state(&self) -> TextFormatterState {
        self.history
            .back()
            .map(|(s, _)| s)
            .cloned()
            .unwrap_or_default()
    }

    pub fn undo(&mut self) -> Option<OutputCommand<EmptyCharIter>> {
        self.latest_suffix = None;
        self.history.pop().map(|(_, undo_info)| {
            debug_assert!(
                undo_info.trailing_suffix.is_none(),
                "orthography-aware undo not implemented yet"
            );
            OutputCommand::Backspace(undo_info.character_count)
        })
    }

    pub fn apply<'s, S: AsRef<str>>(
        &mut self,
        command: &'s FormatterCommand<S>,
    ) -> Option<OutputCommand<impl Iterator<Item = char> + Clone + 's>> {
        use FormatterCommand::*;
        let mut state = self.state();

        let (undo_info, output) = match command {
            Write(input) => {
                // 1. Mutate string according to current state
                let output = state.apply(input.as_ref());
                let output_len = output.clone().count();

                // 2. Advance state
                state.tick();

                // 3. Update the suffix
                let mut suffix = ArrayString::<ORTHOGRAPHIC_SUFFIX_LENGTH>::new();
                let suffix_length = ORTHOGRAPHIC_SUFFIX_LENGTH.min(output_len);

                output
                    .clone()
                    .skip(output_len - suffix_length)
                    .for_each(|c| suffix.push(c));

                self.latest_suffix = Some(suffix);

                // 3. Output result
                (
                    UndoInfo {
                        character_count: output_len as u8,
                        trailing_suffix: None,
                    },
                    Some(OutputCommand::Write(output)),
                )
            }
            ChangeCapitalization(capitalization) => {
                state.capitalization.change_to(*capitalization);
                (UndoInfo::EMPTY, None)
            }
            ChangeAttachment(attachment) => {
                state.attachment.change_to(*attachment);
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
}

impl<const HISTORY_SIZE: usize> Default for Formatter<HISTORY_SIZE> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod does {
    use super::*;
    use crate::output::{OutputAggregator, OutputProcessor};

    // TODO This is a very broad test — figure out a good way to test individual behaviours and do so consistently
    #[test]
    fn work() {
        let mut formatter = Formatter::<10>::new();
        let mut aggregator = OutputAggregator::new();

        let commands = [
            FormatterCommand::Write("hello"),
            FormatterCommand::ChangeCapitalization(CapitalizationMode::Capitalize),
            FormatterCommand::Write("hello"),
            FormatterCommand::ChangeCapitalization(CapitalizationMode::LowerThenCapitalize),
            FormatterCommand::Write("hello"),
            FormatterCommand::ChangeAttachment(AttachmentMode::Always),
            FormatterCommand::Write("world"),
            FormatterCommand::Write("john"),
            FormatterCommand::Write("somethinggone"),
        ];

        for command in commands {
            if let Some(output) = formatter.apply(&command) {
                aggregator.apply(output);
            }
        }

        aggregator.apply(formatter.undo().unwrap());

        assert_eq!(*aggregator, "Hello Hello helloWorldJohn");
    }
}
