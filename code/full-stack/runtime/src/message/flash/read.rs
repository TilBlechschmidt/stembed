use super::{deserialize_data, serialize_data, U24};
use cofit::{Message, MessageIdentifier};

/// Reads a region of memory from flash. Peripheral will emit multiple FlashContent messages that cover the requested range.
/// Additional trailing bytes may be transmitted to fill the remaining space in the last content message.
#[derive(Copy, Clone)]
pub struct ReadFlash<const MTU: usize> {
    pub start: U24,
    pub end: U24,
}

/// Chunk of data that has been read from flash
#[derive(Copy, Clone)]
pub struct FlashContent {
    pub offset: U24,
    pub data: [u8; 63 - 3],
}

impl<const MTU: usize> Message<MTU> for ReadFlash<MTU> {
    const IDENTIFIER: MessageIdentifier<'static> = "flash.read";

    fn to_packet(self) -> [u8; MTU] {
        let mut packet = [0; MTU];
        self.start.write_into(&mut packet[0..3]);
        self.end.write_into(&mut packet[3..6]);
        packet
    }

    fn from_packet(packet: [u8; MTU]) -> Result<Self, ()> {
        let start = U24::from(&packet[0..3]);
        let end = U24::from(&packet[3..6]);
        Ok(Self { start, end })
    }
}

impl Message<63> for FlashContent {
    const IDENTIFIER: MessageIdentifier<'static> = "flash.content";

    fn to_packet(self) -> [u8; 63] {
        serialize_data(self.offset, self.data)
    }

    fn from_packet(packet: [u8; 63]) -> Result<Self, ()> {
        let (offset, data) = deserialize_data(packet);
        Ok(Self { offset, data })
    }
}
