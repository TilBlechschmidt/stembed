use core::ops::Deref;

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
#[repr(transparent)]
pub struct ShortID(u8);

pub type Identifier = &'static str;

pub trait Identifiable {
    const IDENTIFIER: Identifier;
}

impl From<u8> for ShortID {
    fn from(raw: u8) -> Self {
        Self(raw)
    }
}

impl From<ShortID> for u8 {
    fn from(ident: ShortID) -> Self {
        ident.0
    }
}

impl Deref for ShortID {
    type Target = u8;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
