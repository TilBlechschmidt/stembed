use super::{
    message::{self, Message, ASSIGN_IDENTIFIER, RESET_IDENTIFIER},
    Host, IdentifierRegistry, MessageIdentifier, Peripheral, Role, Transport,
};

/// Receiving half of the network stack
pub struct Receiver<'r, 't, const MTU: usize, T: Transport<MTU>, R: Role> {
    registry: &'r IdentifierRegistry<'r, R>,
    transport: &'t T,
    _role: R,
}

impl<'r, 't, const MTU: usize, T: Transport<MTU>, R: Role> Receiver<'r, 't, MTU, T, R> {
    #[doc(hidden)]
    pub fn new(role: R, registry: &'r IdentifierRegistry<'r, R>, transport: &'t T) -> Self {
        Self {
            registry,
            transport,
            _role: role,
        }
    }
}

impl<'r, 't, const MTU: usize, T: Transport<MTU>> Receiver<'r, 't, MTU, T, Host> {
    /// Receives messages over the transport — this function should be polled constantly in a loop to ensure proper operation of the sending half.
    /// It is your responsibility to make sure the loop is iterated at a sufficient interval so that no incoming messages are dropped
    /// (depending on the underlying transports behaviour; many embedded implementations simply drop messages or have only a very small buffer).
    pub async fn recv(&self) -> (MessageIdentifier, [u8; MTU]) {
        loop {
            let (id, packet) = self.transport.recv().await;
            if let Some(identifier) = self.registry.resolve(id) {
                return (identifier, packet);
            } else {
                // TODO print a warning that we received an invalid packet
            }
        }
    }
}

impl<'r, 't, const MTU: usize, T: Transport<MTU>> Receiver<'r, 't, MTU, T, Peripheral> {
    /// Receives messages over the transport — this function should be polled constantly in a loop to ensure proper operation of the sending half.
    /// It is your responsibility to make sure the loop is iterated at a sufficient interval so that no incoming messages are dropped
    /// (depending on the underlying transports behaviour; many embedded implementations simply drop messages or have only a very small buffer).
    pub async fn recv(&self) -> (MessageIdentifier, [u8; MTU]) {
        loop {
            let (id, packet) = self.transport.recv().await;
            if let Some(identifier) = self.registry.resolve(id) {
                match identifier {
                    RESET_IDENTIFIER => self.registry.clear(),
                    ASSIGN_IDENTIFIER => self.handle_assignment(packet),
                    _ => return (identifier, packet),
                }
            } else {
                // TODO print a warning that we received an invalid packet
            }
        }
    }

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
}
