#[cfg(feature = "host")]
use super::message;
use super::{IdentifierRegistry, Message, RegistryLookupResult, Transport};

/// Transmitting half of the network stack
pub struct Transmitter<'r, 't, const MTU: usize, T: Transport<MTU>> {
    registry: &'r IdentifierRegistry<'r>,
    transport: &'t T,
}

impl<'r, 't, const MTU: usize, T: Transport<MTU>> Transmitter<'r, 't, MTU, T> {
    #[doc(hidden)]
    pub fn new(registry: &'r IdentifierRegistry<'r>, transport: &'t T) -> Self {
        Self {
            registry,
            transport,
        }
    }

    /// Attempts to transmit the provided message on the underlying [`Transport`](super::Transport).
    ///
    /// Panics when the message type has not been previously registered while creating the network.
    /// Additionally, the message is dropped if no numeric identifier has been assigned yet.
    pub async fn send<M: Message<MTU>>(&self, message: M) {
        match self.registry.lookup(M::IDENTIFIER) {
            RegistryLookupResult::ID(id) => self.transport.send(id, message.to_packet()).await,
            RegistryLookupResult::Unassigned => {}
            RegistryLookupResult::Unknown => {
                panic!("attempted to send message of type which is not in the registry")
            }
        }
    }

    /// Performs a reset of the remote devices' network stack to establish communication. This should be called whenever you connect or reconnect to a peripheral!
    // TODO Potentially make this a 2-way handshake so we can be sure that the other side received it and is answering as expected
    #[cfg(feature = "host")]
    pub async fn reset_peripheral(&self) {
        self.send(message::Reset).await;
        self.registry.assign_all();
        self.transmit_assignments().await;
    }

    #[cfg(feature = "host")]
    async fn transmit_assignments(&self) {
        for (identifier, id) in self.registry.assign_all() {
            self.send(message::Assign::new(id, identifier)).await;
        }
    }
}
