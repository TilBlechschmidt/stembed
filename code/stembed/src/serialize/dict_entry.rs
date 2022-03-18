use crate::{
    core::{
        dict::binary::{BinaryDictionaryEntry, BinaryDictionaryEntryError},
        engine::Command,
        Stroke, StrokeContext,
    },
    io::{self, Read, ReadExt, Write, WriteExt},
};
use smallvec::SmallVec;

#[derive(Debug)]
pub enum BinaryDictionaryEntrySerializationError {
    IOError(io::Error),
    StrokeUnserializable(io::Error),
    CommandUnserializable(io::Error),
    InvalidData(BinaryDictionaryEntryError),
}

impl<'c> BinaryDictionaryEntry<'c> {
    pub async fn serialize(
        &self,
        writer: &mut impl Write,
    ) -> Result<(), BinaryDictionaryEntrySerializationError> {
        let mut info: u16 = 0;
        info |= (self.tag() & 0b11111) << 11;
        info |= ((self.outline().len() as u16) & 0b11111) << 6;
        info |= (self.commands().len() as u16) & 0b111111;
        writer
            .write_u16(info)
            .await
            .map_err(BinaryDictionaryEntrySerializationError::IOError)?;

        for stroke in self.outline().iter() {
            stroke
                .serialize(writer)
                .await
                .map_err(BinaryDictionaryEntrySerializationError::StrokeUnserializable)?;
        }

        for command in self.commands().iter() {
            command
                .serialize(writer)
                .await
                .map_err(BinaryDictionaryEntrySerializationError::CommandUnserializable)?;
        }

        Ok(())
    }

    pub async fn deserialize(
        reader: &mut impl Read,
        context: &'c StrokeContext,
    ) -> Result<BinaryDictionaryEntry<'c>, BinaryDictionaryEntrySerializationError> {
        let info = reader
            .read_u16()
            .await
            .map_err(BinaryDictionaryEntrySerializationError::IOError)?;
        let tag = info >> 11;
        let stroke_count = (info >> 6) & 0b11111;
        let command_count = info & 0b111111;

        let mut outline = SmallVec::new();
        for _ in 0..stroke_count {
            outline.push(
                Stroke::deserialize(reader, context)
                    .await
                    .map_err(BinaryDictionaryEntrySerializationError::IOError)?,
            );
        }

        let mut commands = SmallVec::new();
        for _ in 0..command_count {
            commands.push(
                Command::deserialize(reader)
                    .await
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
        io::{util::HeapFile, Seek, SeekFrom},
    };
    use smallvec::smallvec;

    #[test]
    fn survive_roundtrip_with_multiple_extra_keys() {
        let context =
            StrokeContext::new("#STKPWHR", "AO*EU", "FRPBLGTSDZ", &["FN1", "FN2"]).unwrap();
        let stroke = Stroke::from_str("KPA*", &context).unwrap();
        let entry = BinaryDictionaryEntry::new(16, smallvec![stroke], smallvec![]).unwrap();

        let mut buf = HeapFile::new();
        smol::block_on(entry.serialize(&mut buf)).unwrap();
        smol::block_on(buf.seek(SeekFrom::Start(0))).unwrap();
        let deserialized =
            smol::block_on(BinaryDictionaryEntry::deserialize(&mut buf, &context)).unwrap();

        assert_eq!(entry.outline(), deserialized.outline());
    }
}
