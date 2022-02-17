use super::{Deserialize, Serialize};
use crate::{
    core::{SharedStrokeContext, Stroke},
    io::{self, Read, Write},
};
use core::hash::{Hash, Hasher};
use smallvec::SmallVec;

impl Serialize for Stroke {
    type Error = io::Error;

    fn serialize(&self, writer: &mut impl Write) -> Result<(), Self::Error> {
        for byte in self.bit_vec.iter() {
            writer.write_u8(*byte)?;
        }

        Ok(())
    }
}

impl Deserialize for Stroke {
    type Context = SharedStrokeContext;
    type Error = io::Error;

    fn deserialize(reader: &mut impl Read, context: &Self::Context) -> Result<Self, Self::Error> {
        let mut bit_vec = SmallVec::new();

        for _ in 0..context.byte_count() {
            bit_vec.push(reader.read_u8()?);
        }

        Ok(Stroke {
            bit_vec,
            context: context.clone(),
        })
    }
}

impl Hash for Stroke {
    fn hash<H>(&self, hasher: &mut H)
    where
        H: Hasher,
    {
        self.serialize(hasher)
            .expect("unknown error while serializing into hasher");
    }
}
