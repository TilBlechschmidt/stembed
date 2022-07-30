mod erase;
mod read;
mod write;

use core::ops::Deref;

pub use erase::{EraseFlash, FlashErased};
pub use read::{FlashContent, ReadFlash};
pub use write::{FlashWritten, WriteFlash};

/// Big-endian 24-bit unsigned integer
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct U24(u32);

impl U24 {
    #[inline]
    fn write_into(self, buffer: &mut [u8]) {
        let bytes: [u8; 3] = self.into();
        buffer[0..3].copy_from_slice(&bytes);
    }
}

impl From<&[u8]> for U24 {
    fn from(bytes: &[u8]) -> Self {
        assert_eq!(bytes.len(), 3);
        Self(u32::from_be_bytes([0, bytes[0], bytes[1], bytes[2]]))
    }
}

impl From<[u8; 3]> for U24 {
    fn from(bytes: [u8; 3]) -> Self {
        Self(u32::from_be_bytes([0, bytes[0], bytes[1], bytes[2]]))
    }
}

impl From<U24> for [u8; 3] {
    fn from(number: U24) -> Self {
        let bytes = number.0.to_be_bytes();
        [bytes[0], bytes[1], bytes[2]]
    }
}

impl From<U24> for u32 {
    fn from(instance: U24) -> Self {
        instance.0
    }
}

impl From<u32> for U24 {
    fn from(number: u32) -> Self {
        Self(number)
    }
}

impl Deref for U24 {
    type Target = u32;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[inline]
fn serialize_data(offset: U24, data: [u8; 63 - 3]) -> [u8; 63] {
    let mut packet = [0; 63];
    offset.write_into(&mut packet[0..3]);
    packet[3..].copy_from_slice(&data);
    packet
}

#[inline]
fn deserialize_data(packet: [u8; 63]) -> (U24, [u8; 63 - 3]) {
    let offset = U24::from(&packet[0..3]);
    let mut data = [0; 63 - 3];
    data.copy_from_slice(&packet[3..]);
    (offset, data)
}
