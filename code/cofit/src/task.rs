use super::{Message, MessageIdentifier};
use core::future::Future;

/// Processor for a single [`Message`](super::Message) type
pub trait Handler<const MTU: usize> {
    /// Message type this handler can process
    type Message: Message<MTU>;

    type RecvFut<'s>: Future + 's
    where
        Self: 's;

    /// Processes a incoming message
    ///
    /// Implementations should not block for any considerable amount of time and the returned future
    /// should complete as soon as possible to avoid dropping incoming messages.
    fn handle<'s>(&'s self, message: Self::Message) -> Self::RecvFut<'s>;

    #[doc(hidden)]
    fn handle_raw<'s>(
        &'s self,
        identifier: MessageIdentifier<'_>,
        packet: &[u8; MTU],
    ) -> Result<Self::RecvFut<'s>, ()> {
        if Self::Message::IDENTIFIER != identifier {
            Err(())
        } else {
            let message = Self::Message::from_packet(*packet)?;
            Ok(self.handle(message))
        }
    }
}

/// Creates an async task that receives messages and calls [`Handler`](self::Handler)s for them
///
/// This macro creates an `async move` block which loops indefinitely, receiving messages and
/// calling the handlers you provided for their corresponding message types.
///
/// Note that you do need to make sure that the messages your handlers can process are registered
/// when you create the network. If you don't, the handlers will never be called!
///
/// # Example
///
/// ```ignore
/// let transport = UsbHidTransport::new();
/// let (tx, rx) = make_network!(&transport, [WriteFlashMessage, ReadFlashMessage]);
///
/// let flash = Flash::new();
/// let write_handler = WriteHandler::new(&flash, &tx);
/// let read_handler = ReadHandler::new(&flash, &tx);
///
/// let rx_task = make_receiver_task!(rx, [write_handler, read_handler]);
/// tokio::spawn(rx_task);
/// ```
#[macro_export]
macro_rules! make_receiver_task {
    ($receiver:expr, [$($handler:expr),+ $(,)?]) => {
        {
            use $crate::Handler;

            // TODO Verify that all handler message types are registered in the $receiver.registry

            async move {
                loop {
                    let (identifier, packet) = $receiver.recv().await;

                    $(
                    if let Ok(handle_fut) = $handler.handle_raw(identifier, &packet) {
                        handle_fut.await;
                    }
                    )+
                }
            }
        }
    };
}
