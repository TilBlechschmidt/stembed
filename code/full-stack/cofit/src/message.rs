use super::{MessageID, MessageIdentifier};

/// Statically allocated ID for resetting all assignments
pub(crate) const RESET_ID: MessageID = MessageID::MAX - 1;
pub(crate) const RESET_IDENTIFIER: MessageIdentifier<'static> = "net.reset";

/// Statically allocated ID for making new assignments
pub(crate) const ASSIGN_ID: MessageID = MessageID::MAX - 2;
pub(crate) const ASSIGN_IDENTIFIER: MessageIdentifier<'static> = "net.assign";

/// Typed data packet sent over the wire, identified by a [`MessageIdentifier`](super::MessageIdentifier)
pub trait Message<const MTU: usize>: Sized {
    /// Unique identifier for this message, this will be transferred over the wire while negotiating the communication protocol!
    const IDENTIFIER: MessageIdentifier<'static>;

    /// Serializes the typed message into a packet of bytes
    fn to_packet(self) -> [u8; MTU];

    /// Deserializes the packet of bytes back into a typed message instance
    #[allow(clippy::result_unit_err)]
    fn from_packet(packet: [u8; MTU]) -> Result<Self, ()>;
}

pub(crate) struct Reset;
pub(crate) struct Assign<const MTU: usize>([u8; MTU]);

impl<const MTU: usize> Message<MTU> for Reset {
    const IDENTIFIER: MessageIdentifier<'static> = RESET_IDENTIFIER;

    fn to_packet(self) -> [u8; MTU] {
        [0; MTU]
    }

    fn from_packet(_: [u8; MTU]) -> Result<Self, ()> {
        Ok(Self)
    }
}

impl<const MTU: usize> Message<MTU> for Assign<MTU> {
    const IDENTIFIER: MessageIdentifier<'static> = ASSIGN_IDENTIFIER;

    fn to_packet(self) -> [u8; MTU] {
        self.0
    }

    fn from_packet(packet: [u8; MTU]) -> Result<Self, ()> {
        if packet[1] < 254 {
            Ok(Self(packet))
        } else {
            Err(())
        }
    }
}

impl<const MTU: usize> Assign<MTU> {
    pub(crate) fn new(id: MessageID, identifier: MessageIdentifier) -> Self {
        let mut buf = [0; MTU];
        buf[0] = id;

        let identifier_bytes = identifier.as_bytes();
        buf[1] = identifier_bytes.len() as u8;
        buf[2..2 + identifier_bytes.len()].copy_from_slice(identifier_bytes);

        Self(buf)
    }

    pub(crate) fn id(&self) -> MessageID {
        self.0[0]
    }

    pub(crate) fn identifier(&self) -> MessageIdentifier {
        let length = self.0[1] as usize;
        let bytes = &self.0[2..2 + length];
        core::str::from_utf8(bytes).unwrap()
    }
}
