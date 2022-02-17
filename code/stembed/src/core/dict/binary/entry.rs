use crate::{
    constants::AVG_STROKE_COUNT,
    core::{dict::CommandList, processor::text_formatter::TextOutputCommand, Stroke},
};
use core::fmt::Debug;
use smallvec::SmallVec;

#[derive(Debug)]
pub enum BinaryDictionaryEntryError {
    /// You passed in a tag that was larger than 31 (5-bit unsigned integer)
    TagTooLarge,
    /// You passed more than 32 (5-bit unsigned integer) strokes
    TooManyStrokes,
    /// You passed more than 64 (6-bit unsigned integer) commands
    TooManyCommands,
}

pub type Outline = SmallVec<[Stroke; AVG_STROKE_COUNT]>;

pub(crate) struct BinaryDictionaryEntry {
    tag: u16,
    outline: Outline,
    commands: CommandList<TextOutputCommand>,
}

impl BinaryDictionaryEntry {
    pub(crate) fn new(
        tag: u16,
        outline: Outline,
        commands: CommandList<TextOutputCommand>,
    ) -> Result<Self, BinaryDictionaryEntryError> {
        if tag > 32 {
            Err(BinaryDictionaryEntryError::TagTooLarge)
        } else if outline.len() > 32 {
            Err(BinaryDictionaryEntryError::TooManyStrokes)
        } else if commands.len() > 64 {
            Err(BinaryDictionaryEntryError::TooManyCommands)
        } else {
            Ok(Self {
                tag,
                outline,
                commands,
            })
        }
    }

    pub fn tag(&self) -> u16 {
        self.tag
    }

    pub fn outline(&self) -> &Outline {
        &self.outline
    }

    pub fn commands(&self) -> &CommandList<TextOutputCommand> {
        &self.commands
    }

    pub fn into_commands(self) -> CommandList<TextOutputCommand> {
        self.commands
    }
}
