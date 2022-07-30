#![allow(clippy::needless_lifetimes)]

use super::MessageID;
use core::future::Future;

/// Physical data transfer layer
///
/// Asynchronously transmits and receives packets over a given physical medium like a USB wire or Bluetooth wireless connection.
pub trait Transport<const MTU: usize> {
    type TxFut<'t>: Future<Output = ()> + 't
    where
        Self: 't;

    type RxFut<'t>: Future<Output = (MessageID, [u8; MTU])> + 't
    where
        Self: 't;

    /// Sends a message over the wire/air
    ///
    /// It is recommended that implementations of this function maintain a small internal buffer
    /// so that immediate same-task responses to incoming messages can send without
    /// soft-blocking reception of additional messages.
    fn send<'t>(&'t self, id: MessageID, data: [u8; MTU]) -> Self::TxFut<'t>;

    /// Receives a message over the wire/air
    ///
    /// The implementation may drop packets when the RxFut is not polled while the packet arrives,
    /// though it is recommended that the transport maintains a small internal buffer to allow for
    /// minor lags while processing messages.
    fn recv<'t>(&'t self) -> Self::RxFut<'t>;
}
