use crate::io::{Error as IOError, Read, Write};
use core::future::Future;
use smol_str::SmolStr;

#[derive(Debug)]
pub enum StringSerializationError {
    LengthOverflow,
    InvalidData(core::str::Utf8Error),
    IOError(IOError),
}

pub trait SmolStrExt: Sized {
    type Error;

    type SerializeFuture<'a, W>: Future<Output = Result<(), Self::Error>> + 'a
    where
        Self: 'a,
        W: 'a + Write;

    type DeserializeFuture<'a, R>: Future<Output = Result<Self, Self::Error>> + 'a
    where
        R: 'a + Read;

    fn serialize<'a, W: Write>(&'a self, writer: &'a mut W) -> Self::SerializeFuture<'a, W>;
    fn deserialize<'a, R: Read>(reader: &'a mut R) -> Self::DeserializeFuture<'a, R>;
}

impl SmolStrExt for SmolStr {
    type Error = StringSerializationError;
    type SerializeFuture<'a, W> = impl Future<Output = Result<(), Self::Error>> + 'a where Self: 'a, W: 'a + Write;
    type DeserializeFuture<'a, R> = impl Future<Output = Result<Self, Self::Error>> + 'a
    where
        Self: 'a,
        R: 'a + Read;

    /// Default implementation using length-prefixed strings
    fn serialize<'a, W: Write>(&'a self, writer: &'a mut W) -> Self::SerializeFuture<'a, W> {
        async move {
            if self.len() > u8::MAX as usize {
                Err(StringSerializationError::LengthOverflow)
            } else {
                // Write the length as one byte
                writer
                    .write(self.len() as u8)
                    .await
                    .map_err(StringSerializationError::IOError)?;

                // Write the raw string data
                for byte in self.as_bytes() {
                    writer
                        .write(*byte)
                        .await
                        .map_err(StringSerializationError::IOError)?;
                }

                Ok(())
            }
        }
    }

    fn deserialize<'a, R: Read>(reader: &'a mut R) -> Self::DeserializeFuture<'a, R> {
        async move {
            let mut data = [0u8; u8::MAX as usize];
            let length = reader
                .read()
                .await
                .map_err(StringSerializationError::IOError)? as usize;

            for i in 0..length {
                data[i] = reader
                    .read()
                    .await
                    .map_err(StringSerializationError::IOError)?;
            }

            let string = core::str::from_utf8(&data[0..length])
                .map_err(StringSerializationError::InvalidData)?;
            Ok(SmolStr::new(string))
        }
    }
}
