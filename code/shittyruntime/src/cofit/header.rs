use serde::{Deserialize, Serialize};

/// 6-bit identifier
#[derive(PartialEq, Eq, Clone, Copy, Serialize, Deserialize, Debug)]
#[repr(transparent)]
pub struct ID(u8);

#[derive(Debug)]
pub enum PacketHeaderParseError {
    UnknownPacketType,
}

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum PacketHeader {
    Message(ID),
    MessageAck(ID),
    StreamPacket(StreamPacketHeader),
}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum StreamPacketHeader {
    Content(u8),
    Revert,
    Closed,
}

impl From<u8> for ID {
    fn from(src: u8) -> Self {
        assert!(src < 2u8.pow(6));
        Self(src)
    }
}

impl From<ID> for u8 {
    fn from(src: ID) -> Self {
        src.0
    }
}

impl TryFrom<u8> for PacketHeader {
    type Error = PacketHeaderParseError;

    fn try_from(src: u8) -> Result<Self, Self::Error> {
        use PacketHeader::*;
        use PacketHeaderParseError::*;
        use StreamPacketHeader::*;

        let header = if src & 0b11_000000 == 0 {
            Message(ID::from(src))
        } else if src & 0b11_000000 == 0b01_000000 {
            MessageAck(ID::from(src & 0b111111))
        } else if src & 0b11_000000 == 0b10_000000 {
            StreamPacket(Content(src & 0b111111))
        } else if src == 0b11000000 {
            StreamPacket(Revert)
        } else if src == 0b11000001 {
            StreamPacket(Closed)
        } else {
            return Err(UnknownPacketType);
        };

        Ok(header)
    }
}

impl From<PacketHeader> for u8 {
    fn from(src: PacketHeader) -> Self {
        use PacketHeader::*;
        use StreamPacketHeader::*;
        match src {
            Message(id) => id.into(),
            MessageAck(id) => 0b01_000000 | <ID as Into<u8>>::into(id),
            StreamPacket(Content(id)) => 0b10_000000 | id,
            StreamPacket(Revert) => 0b11000000,
            StreamPacket(Closed) => 0b11000001,
        }
    }
}
