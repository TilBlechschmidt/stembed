#![feature(generic_associated_types)]
#![feature(type_alias_impl_trait)]

use cofit::{
    make_network, make_receiver_task, Handler, Host, Message, MessageIdentifier, Transport,
};
use core::future::Future;

const MTU: usize = 42;

struct DummyTransport;

impl Transport<MTU> for DummyTransport {
    type TxFut<'t> = impl Future<Output = ()> + 't
    where
        Self: 't;

    type RxFut<'t> = impl Future<Output = (u8, [u8; MTU])> + 't
    where
        Self: 't;

    fn send<'t>(&'t self, id: u8, data: [u8; MTU]) -> Self::TxFut<'t> {
        async move { unimplemented!() }
    }

    fn recv<'t>(&'t self) -> Self::RxFut<'t> {
        async move { unimplemented!() }
    }
}

struct MessageA;

impl Message<MTU> for MessageA {
    const IDENTIFIER: MessageIdentifier<'static> = "dummy.a";

    fn to_packet(self) -> [u8; MTU] {
        unimplemented!()
    }

    fn from_packet(packet: [u8; MTU]) -> Result<Self, ()> {
        unimplemented!()
    }
}

struct MessageB;

impl Message<MTU> for MessageB {
    const IDENTIFIER: MessageIdentifier<'static> = "dummy.b";

    fn to_packet(self) -> [u8; MTU] {
        unimplemented!()
    }

    fn from_packet(packet: [u8; MTU]) -> Result<Self, ()> {
        unimplemented!()
    }
}

struct MessageAHandler;

impl Handler<MTU> for MessageAHandler {
    type Message = MessageA;

    type RecvFut<'s> = impl Future + 's
    where
        Self: 's;

    fn handle<'s>(&'s self, message: Self::Message) -> Self::RecvFut<'s> {
        async move { unimplemented!() }
    }
}

struct MessageBHandler;

impl Handler<MTU> for MessageBHandler {
    type Message = MessageB;

    type RecvFut<'s> = impl Future + 's
    where
        Self: 's;

    fn handle<'s>(&'s self, message: Self::Message) -> Self::RecvFut<'s> {
        async move { unimplemented!() }
    }
}

#[test]
fn it_does_stuff() {
    let transport = DummyTransport;
    let handler_a = MessageAHandler;
    let handler_b = MessageBHandler;

    let (tx, rx) = make_network! {
        role: Host,
        transport: &transport,
        messages: [MessageA, MessageB]
    };
    let rx_task = make_receiver_task!(rx, [handler_a, handler_b]);
}
