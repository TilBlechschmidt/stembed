use super::{MpscReceiver, StreamPacketHeader, Transport};
use serde::{Deserialize, Serialize};

mod read;
mod write;

pub use read::StreamReadHandle;
pub use write::StreamWriteHandle;

/// 22-bit stream section identifier
#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Copy, PartialOrd, Ord)]
#[repr(transparent)]
pub struct StreamSequenceID(u32);

impl StreamSequenceID {
    const MAX_VALUE: u32 = 2u32.pow(6 + 8 + 8);

    fn increment(&mut self) {
        self.0 += 1;
        assert!(self.0 < Self::MAX_VALUE);
    }

    fn into_offset(self, smtu: usize) -> u64 {
        self.0 as u64 * smtu as u64
    }

    fn into_bytes(self) -> [u8; 3] {
        let bytes = self.0.to_be_bytes();
        [bytes[1], bytes[2], bytes[3]]
    }

    fn from_bytes(bytes: [u8; 3]) -> Self {
        Self::from(u32::from_be_bytes([0, bytes[0], bytes[1], bytes[2]]))
    }
}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub struct StreamPacket<const MTU: usize> {
    pub(super) header: StreamPacketHeader,
    pub(super) bytes: [u8; MTU],
}

#[derive(Serialize, Deserialize)]
struct StreamClosePacket {
    /// Last sequence ID that was part of the stream
    sequence_id: StreamSequenceID,
}

#[derive(Serialize, Deserialize)]
struct StreamRevertPacket {
    /// Sequence ID from which data should be retransmitted (inclusive)
    sequence_id: StreamSequenceID,
}

impl From<u32> for StreamSequenceID {
    fn from(src: u32) -> Self {
        assert!(src < Self::MAX_VALUE);
        Self(src)
    }
}

impl From<StreamSequenceID> for u32 {
    fn from(src: StreamSequenceID) -> Self {
        src.0
    }
}
