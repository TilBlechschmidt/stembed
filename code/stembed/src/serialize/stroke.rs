use crate::{
    core::{Stroke, StrokeContext},
    io::{self, Read, Write},
};
use core::hash::{Hash, Hasher};
use smallvec::SmallVec;

impl<'c> Stroke<'c> {
    pub async fn serialize(&self, writer: &mut impl Write) -> Result<(), io::Error> {
        for byte in self.bit_vec.iter() {
            writer.write(*byte).await?;
        }

        Ok(())
    }

    pub async fn deserialize<'a>(
        reader: &mut impl Read,
        context: &'a StrokeContext,
    ) -> Result<Stroke<'a>, io::Error> {
        let mut bit_vec = SmallVec::new();

        for _ in 0..context.byte_count() {
            bit_vec.push(reader.read().await?);
        }

        Ok(Stroke { bit_vec, context })
    }
}

impl<'c> Hash for Stroke<'c> {
    fn hash<H>(&self, hasher: &mut H)
    where
        H: Hasher,
    {
        hasher.write(self.bit_vec.as_slice());
    }
}
