use smallvec::SmallVec;

use crate::{
    core::{
        dict::binary::{BinaryDictionaryEntry, BinaryDictionaryEntryError},
        engine::Command,
        processor::text_formatter::TextOutputCommand,
        Stroke,
    },
    io,
};

use super::{Deserialize, Serialize};

#[derive(Debug)]
pub enum BinaryDictionaryEntrySerializationError {
    IOError(io::Error),
    StrokeUnserializable(<Stroke as Serialize>::Error),
    CommandUnserializable(<Command<TextOutputCommand> as Serialize>::Error),
    InvalidData(BinaryDictionaryEntryError),
}

impl Serialize for BinaryDictionaryEntry {
    type Error = BinaryDictionaryEntrySerializationError;

    fn serialize(&self, writer: &mut impl io::Write) -> Result<(), Self::Error> {
        let mut info: u16 = 0;
        info |= (self.tag() & 0b11111) << 11;
        info |= ((self.outline().len() as u16) & 0b11111) << 6;
        info |= (self.commands().len() as u16) & 0b111111;
        writer.write_u16(info).map_err(Self::Error::IOError)?;

        for stroke in self.outline().iter() {
            stroke
                .serialize(writer)
                .map_err(Self::Error::StrokeUnserializable)?;
        }

        for command in self.commands().iter() {
            command
                .serialize(writer)
                .map_err(Self::Error::CommandUnserializable)?;
        }

        Ok(())
    }
}

impl Deserialize for BinaryDictionaryEntry {
    type Context = <Stroke as Deserialize>::Context;
    type Error = BinaryDictionaryEntrySerializationError;

    fn deserialize(
        reader: &mut impl io::Read,
        context: &Self::Context,
    ) -> Result<Self, Self::Error> {
        let info = reader
            .read_u16()
            .map_err(BinaryDictionaryEntrySerializationError::IOError)?;
        let tag = info >> 11;
        let stroke_count = (info >> 6) & 0b11111;
        let command_count = info & 0b111111;

        let mut outline = SmallVec::new();
        for _ in 0..stroke_count {
            outline.push(
                Stroke::deserialize(reader, &context)
                    .map_err(BinaryDictionaryEntrySerializationError::IOError)?,
            );
        }

        let mut commands = SmallVec::new();
        for _ in 0..command_count {
            commands.push(
                Command::deserialize(reader, &())
                    .map_err(BinaryDictionaryEntrySerializationError::IOError)?,
            );
        }

        Self::new(tag, outline, commands)
            .map_err(BinaryDictionaryEntrySerializationError::InvalidData)
    }
}
