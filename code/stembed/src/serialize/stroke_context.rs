use smallvec::SmallVec;
use smol_str::SmolStr;

use super::{Deserialize, Serialize, StringSerializationError};
use crate::{
    core::StrokeContext,
    io::{Read, Write},
};

impl Serialize for StrokeContext {
    type Error = StringSerializationError;

    fn serialize(&self, writer: &mut impl Write) -> Result<(), Self::Error> {
        // Serialize the simple strings
        self.left.serialize(writer)?;
        self.middle.serialize(writer)?;
        self.right.serialize(writer)?;

        // Serialize the number of extra commands
        writer
            .write_u16(self.extra.len() as u16)
            .map_err(StringSerializationError::IOError)?;

        // Serialize each extra command
        for string in self.extra.iter() {
            string.serialize(writer)?;
        }

        Ok(())
    }
}

impl Deserialize for StrokeContext {
    type Context = ();
    type Error = StringSerializationError;

    fn deserialize(reader: &mut impl Read, context: &Self::Context) -> Result<Self, Self::Error> {
        let left = SmolStr::deserialize(reader, context)?;
        let middle = SmolStr::deserialize(reader, context)?;
        let right = SmolStr::deserialize(reader, context)?;

        let extra_count = reader
            .read_u16()
            .map_err(StringSerializationError::IOError)?;

        let mut extra = SmallVec::new();
        for _ in 0..extra_count {
            extra.push(SmolStr::deserialize(reader, context)?);
        }

        Ok(Self {
            left,
            middle,
            right,
            extra,
        })
    }
}
