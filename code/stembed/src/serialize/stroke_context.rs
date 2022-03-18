use smallvec::SmallVec;
use smol_str::SmolStr;

use super::{SmolStrExt, StringSerializationError};
use crate::{
    core::StrokeContext,
    io::{Read, ReadExt, Write, WriteExt},
};

impl StrokeContext {
    pub async fn serialize<'a>(
        &'a self,
        writer: &'a mut impl Write,
    ) -> Result<(), StringSerializationError> {
        // Serialize the simple strings
        self.left.serialize(writer).await?;
        self.middle.serialize(writer).await?;
        self.right.serialize(writer).await?;

        // Serialize the number of extra commands
        writer
            .write_u16(self.extra.len() as u16)
            .await
            .map_err(StringSerializationError::IOError)?;

        // Serialize each extra command
        for string in self.extra.iter() {
            string.serialize(writer).await?;
        }

        Ok(())
    }

    pub async fn deserialize(reader: &mut impl Read) -> Result<Self, StringSerializationError> {
        let left = SmolStr::deserialize(reader).await?;
        let middle = SmolStr::deserialize(reader).await?;
        let right = SmolStr::deserialize(reader).await?;

        let extra_count = reader
            .read_u16()
            .await
            .map_err(StringSerializationError::IOError)?;

        let mut extra = SmallVec::new();
        for _ in 0..extra_count {
            extra.push(SmolStr::deserialize(reader).await?);
        }

        Ok(Self {
            left,
            middle,
            right,
            extra,
        })
    }
}
