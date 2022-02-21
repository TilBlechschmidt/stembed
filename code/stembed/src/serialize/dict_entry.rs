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
pub enum BinaryDictionaryEntrySerializationError<'c> {
    IOError(io::Error),
    StrokeUnserializable(<Stroke<'c> as Serialize>::Error),
    CommandUnserializable(<Command<TextOutputCommand> as Serialize>::Error),
    InvalidData(BinaryDictionaryEntryError),
}

impl<'c> Serialize for BinaryDictionaryEntry<'c> {
    type Error = BinaryDictionaryEntrySerializationError<'c>;

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

impl<'c> Deserialize for BinaryDictionaryEntry<'c> {
    type Context = <Stroke<'c> as Deserialize>::Context;
    type Error = BinaryDictionaryEntrySerializationError<'c>;

    fn deserialize(
        reader: &mut impl io::Read,
        context: Self::Context,
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
                Stroke::deserialize(reader, context)
                    .map_err(BinaryDictionaryEntrySerializationError::IOError)?,
            );
        }

        let mut commands = SmallVec::new();
        for _ in 0..command_count {
            commands.push(
                Command::deserialize(reader, ())
                    .map_err(BinaryDictionaryEntrySerializationError::IOError)?,
            );
        }

        Self::new(tag, outline, commands)
            .map_err(BinaryDictionaryEntrySerializationError::InvalidData)
    }
}

#[cfg(test)]
mod does {
    use crate::{
        core::{dict::binary::BinaryDictionaryEntry, Stroke, StrokeContext},
        io::{HeapFile, Seek, SeekFrom},
        serialize::{Deserialize, Serialize},
    };
    use smallvec::smallvec;

    #[test]
    fn survive_roundtrip_with_multiple_extra_keys() {
        let context =
            StrokeContext::new("#STKPWHR", "AO*EU", "FRPBLGTSDZ", &["FN1", "FN2"]).unwrap();
        let stroke = Stroke::from_str("KPA*", &context).unwrap();
        let entry = BinaryDictionaryEntry::new(16, smallvec![stroke], smallvec![]).unwrap();

        let mut buf = HeapFile::new();
        entry.serialize(&mut buf).unwrap();
        buf.seek(SeekFrom::Start(0)).unwrap();
        let deserialized = BinaryDictionaryEntry::deserialize(&mut buf, &context).unwrap();

        assert_eq!(entry.outline(), deserialized.outline());
    }
}
