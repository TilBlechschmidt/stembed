#[cfg(feature = "peripheral")]
use super::message::{self, Message};
use super::{
    message::{ASSIGN_IDENTIFIER, RESET_IDENTIFIER},
    IdentifierRegistry, MessageIdentifier, Transport,
};

/// Receiving half of the network stack
pub struct Receiver<'r, 't, const MTU: usize, T: Transport<MTU>> {
    registry: &'r IdentifierRegistry<'r>,
    transport: &'t T,
}

impl<'r, 't, const MTU: usize, T: Transport<MTU>> Receiver<'r, 't, MTU, T> {
    #[doc(hidden)]
    pub fn new(registry: &'r IdentifierRegistry<'r>, transport: &'t T) -> Self {
        Self {
            registry,
            transport,
        }
    }

    /// Receives messages over the transport — this function should be polled constantly in a loop to ensure proper operation of the sending half.
    /// It is your responsibility to make sure the loop is iterated at a sufficient interval so that no incoming messages are dropped
    /// (depending on the underlying transports behaviour; many embedded implementations simply drop messages or have only a very small buffer).
    pub async fn recv(&self) -> (MessageIdentifier, [u8; MTU]) {
        loop {
            let (id, packet) = self.transport.recv().await;
            if let Some(identifier) = self.registry.resolve(id) {
                match identifier {
                    RESET_IDENTIFIER => self.handle_reset(),
                    ASSIGN_IDENTIFIER => self.handle_assignment(packet),
                    _ => return (identifier, packet),
                }
            } else {
                // TODO print a warning that we received an invalid packet
            }
        }
    }

    fn handle_reset(&self) {
        #[cfg(feature = "peripheral")]
        self.registry.clear();
    }

    #[cfg(feature = "peripheral")]
    fn handle_assignment(&self, packet: [u8; MTU]) {
        if let Ok(assignment) = message::Assign::from_packet(packet) {
            self.registry
                .assign(assignment.id(), assignment.identifier());
            // TODO Send a message that notifies the host that this message type is not supported
            //      so that the host may unassign it again (thus never sending messages of this type).
        } else {
            // TODO Print a warning that we received an invalid assignment
        }
    }

    #[cfg(not(feature = "peripheral"))]
    fn handle_assignment(&self, _: [u8; MTU]) {}
}
