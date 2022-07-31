use super::{deserialize_data, serialize_data, U24};
use cofit::{Message, MessageIdentifier};

/// Writes a region of memory to flash without erasing, requires proper alignment.
#[repr(C, align(4))]
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct WriteFlash {
    pub data: [u8; 63 - 3],
    pub offset: U24,
}

/// Acknowledges a write message and confirms that the data has been written
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct FlashWritten {
    pub data: [u8; 63 - 3],
    pub offset: U24,
}

impl Message<63> for WriteFlash {
    const IDENTIFIER: MessageIdentifier<'static> = "flash.write";

    fn to_packet(self) -> [u8; 63] {
        serialize_data(self.offset, self.data)
    }

    fn from_packet(packet: [u8; 63]) -> Result<Self, ()> {
        let (offset, data) = deserialize_data(packet);
        Ok(Self { offset, data })
    }
}

impl Message<63> for FlashWritten {
    const IDENTIFIER: MessageIdentifier<'static> = "flash.write.ack";

    fn to_packet(self) -> [u8; 63] {
        serialize_data(self.offset, self.data)
    }

    fn from_packet(packet: [u8; 63]) -> Result<Self, ()> {
        let (offset, data) = deserialize_data(packet);
        Ok(Self { offset, data })
    }
}

impl From<WriteFlash> for FlashWritten {
    fn from(write: WriteFlash) -> Self {
        Self {
            offset: write.offset,
            data: write.data,
        }
    }
}

impl From<FlashWritten> for WriteFlash {
    fn from(written: FlashWritten) -> Self {
        Self {
            offset: written.offset,
            data: written.data,
        }
    }
}
