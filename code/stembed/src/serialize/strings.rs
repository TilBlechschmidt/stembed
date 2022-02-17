use crate::io::{Error as IOError, Read, Write};
use smol_str::SmolStr;

use super::{Deserialize, Serialize};

#[derive(Debug)]
pub enum StringSerializationError {
    LengthOverflow,
    InvalidData(core::str::Utf8Error),
    IOError(IOError),
}

impl Serialize for SmolStr {
    type Error = StringSerializationError;

    /// Default implementation using length-prefixed strings
    fn serialize(&self, writer: &mut impl Write) -> Result<(), Self::Error> {
        if self.len() > u8::MAX as usize {
            Err(StringSerializationError::LengthOverflow)
        } else {
            // Write the length as one byte
            writer
                .write_u8(self.len() as u8)
                .map_err(StringSerializationError::IOError)?;

            // Write the raw string data
            for byte in self.as_bytes() {
                writer
                    .write_u8(*byte)
                    .map_err(StringSerializationError::IOError)?;
            }

            Ok(())
        }
    }
}

impl Deserialize for SmolStr {
    type Context = ();
    type Error = StringSerializationError;

    fn deserialize(reader: &mut impl Read, _context: &Self::Context) -> Result<Self, Self::Error> {
        let mut data = [0u8; u8::MAX as usize];
        let length = reader
            .read_u8()
            .map_err(StringSerializationError::IOError)? as usize;

        for i in 0..length {
            data[i] = reader
                .read_u8()
                .map_err(StringSerializationError::IOError)?;
        }

        let string = core::str::from_utf8(&data[0..length])
            .map_err(StringSerializationError::InvalidData)?;
        Ok(SmolStr::new(string))
    }
}
