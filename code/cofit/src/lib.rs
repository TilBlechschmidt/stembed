//! # Communication over fixed interval transports
//!
//! This crate provides a networking layer for communication over transports like USB HID or Bluetooth Low Energy.
//! These transports have fixed frame sizes and always transmit data at fixed intervals — 1ms or 30ms respectively.
//!
//! The primary goal of this library is to handle transmission of different message types over the same wire
//! while making sure that both the host and peripheral have a mutual understanding of each others capabilities
//! and ensuring that messages the other side can not handle won't be transmitted.
//!
//! Additionally, it provides a macro for creating a message receiver loop and some additional tooling for
//! handling messages whose processing might take longer asynchronously.
//!
//! ## Message identifier assignment
//!
//! When creating a network, you provide a list of messages that make up the protocol your instance supports.
//! This holds true for both the host and peripheral roles.
//!
//! A host device assigns numeric identifiers to every message type it supports and communicates these to the peripheral.
//! The peripheral in turn remembers these assignments — but only for those it can support. This way, the two devices
//! may have different supported features (e.g. through version mismatches) but still communicate based on the messages
//! they have in common.
//!
//! Additionally, assigning numeric identifiers allows for more efficient transfers since the MTU is usually very low
//! and the overhead of transfering a dynamic-size string identifier with each is not tolerable.
//!
//! The peripheral will never send messages which are not supported by the host and will ignore any messages sent by the host
//! that it can not handle (in the future the host may be notified of the peripherals capabilities so those messages wont even be sent).
//!
//! ## Feature flags
//!
//! | Flag | Features |
//! |--------|----------|
//! | `host` | Asserts message identifier assignment authority |
//! | `peripheral` | Slave mode for message identifier assignments |
//!
//! ## Usage workflow
//!
//! 1. Create a [`Transport`](self::Transport) implementation
//! 2. Write some [`Message`](self::Message) trait implementations
//! 3. Author some [`Handler`](self::Handler) impls for your message types
//! 3. Initialize a [`Receiver`](self::Receiver) + [`Transmitter`](self::Transmitter) pair using [`make_network!`](self::make_network)
//! 4. Build a receiver task using [`make_receiver_task!`](self::make_receiver_task)
//! 5. Poll the task for all eternity
//! 6. If you are the host, call [`reset_peripheral`](self::Transmitter::reset_peripheral) once connected
//! 7. Send some messages!

#![no_std]
#![feature(generic_associated_types)]

#[cfg(all(feature = "host", feature = "peripheral"))]
compile_error!("features `cofit/host` and `cofit/peripheral` are mutually exclusive");

type MessageID = u8;

/// Globally unique string identifier for a message
///
/// It is considered best practice to use namespaced strings like `flash.write` or `bluetooth.enable` to make collisions unlikely.
/// If you are writing a vendor specific extension, consider using your domain as a prefix.
pub type MessageIdentifier<'i> = &'i str;

mod message;
mod receiver;
mod registry;
mod task;
mod transmitter;
mod transport;

pub use message::Message;
pub use receiver::*;
pub use registry::*;
pub use task::*;
pub use transmitter::*;
pub use transport::*;

/// Creates a new [`Receiver`](self::Receiver) + [`Transmitter`](self::Transmitter) pair from a given transport
///
/// # ⚠️ Static memory allocation
///
/// Note that the macro creates a new static variable for the numeric message identifier assignments! While you can freely drop the transmitter/receiver, these
/// static variables will persist. Thus you shall only ever call this function **ONCE** for a given transport or risk leaking unused memory.
///
/// # Example
///
/// ```
/// # #![feature(generic_associated_types)]
/// # #![feature(type_alias_impl_trait)]
/// # use cofit::{Message, MessageIdentifier, make_network, Transport};
/// # use core::future::Future;
/// # const MTU: usize = 42;
/// #
/// struct WriteFlashMessage; // + impl Message<_> for WriteFlashMessage { ... }
/// struct ReadFlashMessage;  // + impl Message<_> for ReadFlashMessage { ... }
/// #
/// # impl Message<MTU> for WriteFlashMessage {
/// #     const IDENTIFIER: MessageIdentifier<'static> = "flash.write";
/// #
/// #     fn to_packet(self) -> [u8; MTU] {
/// #         unimplemented!()
/// #     }
/// #
/// #     fn from_packet(packet: [u8; MTU]) -> Result<Self, ()> {
/// #         unimplemented!()
/// #     }
/// # }
/// #
/// # impl Message<MTU> for ReadFlashMessage {
/// #     const IDENTIFIER: MessageIdentifier<'static> = "flash.read";
/// #
/// #     fn to_packet(self) -> [u8; MTU] {
/// #         unimplemented!()
/// #     }
/// #
/// #     fn from_packet(packet: [u8; MTU]) -> Result<Self, ()> {
/// #         unimplemented!()
/// #     }
/// # }
///
/// struct UsbHidTransport; // + impl Transport<_> for UsbHidTransport { ... }
/// #
/// # impl UsbHidTransport {
/// #     fn new() -> Self {
/// #         Self
/// #     }
/// # }
/// #
/// # impl Transport<MTU> for UsbHidTransport {
/// #     type TxFut<'t> = impl Future<Output = ()> + 't
/// #     where
/// #         Self: 't;
/// #
/// #     type RxFut<'t> = impl Future<Output = (u8, [u8; MTU])> + 't
/// #     where
/// #         Self: 't;
/// #
/// #     fn send<'t>(&'t self, id: u8, data: [u8; MTU]) -> Self::TxFut<'t> {
/// #         async move { unimplemented!() }
/// #     }
/// #
/// #     fn recv<'t>(&'t self) -> Self::RxFut<'t> {
/// #         async move { unimplemented!() }
/// #     }
/// # }
///
/// let transport = UsbHidTransport::new();
/// let (tx, rx) = make_network!(&transport, [WriteFlashMessage, ReadFlashMessage]);
///
/// // Make sure to constantly call `rx.recv()` so that `tx.send(_)` operates correctly.
/// // You may use `make_receiver_task` to create an async task that does this for you!
/// ```
#[macro_export]
macro_rules! make_network {
    ($transport:expr, [$($message:ty),+ $(,)?]) => {
        {
            use $crate::{make_network, IdentifierRegistry, Transmitter, Receiver};

            const _: () = IdentifierRegistry::verify_message_count(make_network!(@count $({$message})*));
            // TODO Verify that there are no duplicate messages at compile time (requires sub-macros that emit a compile_error! as recursion/loops are not possible in const code)

            static ASSIGNMENTS: [(core::sync::atomic::AtomicU8, $crate::MessageIdentifier<'static>); make_network!(@count $({$message})*)] = [$((core::sync::atomic::AtomicU8::new(IdentifierRegistry::UNASSIGNED), <$message>::IDENTIFIER),)+];
            static REGISTRY: IdentifierRegistry = IdentifierRegistry::new(&ASSIGNMENTS);

            let transmitter = Transmitter::new(&REGISTRY, $transport);
            let receiver = Receiver::new(&REGISTRY, $transport);

            (transmitter, receiver)
        }
    };

    (@count) => { 0 };
    (@count $t:tt $($rest:tt)*) => { 1 + make_network!(@count $($rest)*) }
}
