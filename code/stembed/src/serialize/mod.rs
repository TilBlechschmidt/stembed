use super::io::{Read, Write};

mod strings;
pub use strings::*;

mod dict_entry;
pub use dict_entry::*;

mod command;
mod stroke;
mod stroke_context;

pub trait Serialize: Sized {
    type Error;

    fn serialize(&self, writer: &mut impl Write) -> Result<(), Self::Error>;
}

pub trait Deserialize: Sized {
    type Context;
    type Error;

    fn deserialize(reader: &mut impl Read, context: &Self::Context) -> Result<Self, Self::Error>;
}
