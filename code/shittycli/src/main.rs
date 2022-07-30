#![feature(type_alias_impl_trait)]
#![feature(generic_associated_types)]

use core::future::Future;
use hid::UsbHidTransport;
use hidapi::HidApi;
use shittyruntime::{
    cofit::{MessageAcknowledger, MessageHandler, Transport, UsbNetwork},
    firmware::{executor_support::Channel, Mpsc},
    messaging::{DataRange, Message, TestFormat},
};
use tokio::select;

mod hid;

const USAGE_PAGE_VENDOR: u16 = 0xFF00;
const USAGE_EMBEDDED_STENO: u16 = 0x42;
const DEVICE_VID: u16 = 0xC0DE;
const DEVICE_PID: u16 = 0xCAFE;

#[tokio::main]
async fn main() {
    let api = HidApi::new().expect("failed to setup HID API");
    let device = api
        .device_list()
        .filter(|d| d.vendor_id() == DEVICE_VID && d.product_id() == DEVICE_PID)
        .filter(|d| d.usage_page() == USAGE_PAGE_VENDOR && d.usage() == USAGE_EMBEDDED_STENO)
        .map(|d| d.open_device(&api))
        .next()
        .expect("no device found")
        .expect("failed to open device");

    let transport = UsbHidTransport::new(device);

    let ack_channel = Channel::new();
    let stream_channel = Channel::new();
    let message_channel = Channel::new();

    let network = UsbNetwork::new(
        transport,
        TestFormat,
        ack_channel.split(),
        stream_channel.split(),
        message_channel.split(),
    );

    println!("opened network");

    // let handler = TestMessageHandler { network: &network };
    let recv_fut = network.recv_task();

    let send_fut = async {
        const read_size: usize = 4096 * 8;

        let mut reader = network.create_stream_reader().await;

        network
            .send(Message::ReadFlash(DataRange {
                offset: 0,
                length: read_size as u64,
            }))
            .await
            .expect("failed to send message"); // TODO This times out (?)

        println!("sent read request");

        let mut buffer = Vec::with_capacity(read_size);
        while let Some(data) = reader.recv().await {
            buffer.extend_from_slice(&data);
        }

        println!("received buffer! {}", buffer.len());
        dbg!(buffer);
    };

    select! {
        _ = recv_fut => {},
        _ = send_fut => {},
    };
}

pub struct TestMessageHandler<'d, T: Transport<64>> {
    pub network: &'d UsbNetwork<'d, T, TestFormat>,
}

impl<'d, T: Transport<64>> TestMessageHandler<'d, T> {
    pub fn new(network: &'d UsbNetwork<'d, T, TestFormat>) -> Self {
        Self { network }
    }
}

impl<'d, T: Transport<64>, const MTU: usize> MessageHandler<Message, MTU>
    for TestMessageHandler<'d, T>
where
    T: 'd,
{
    type HandlerFut<'s> = impl Future<Output = ()> + 's
    where
        Self: 's;

    fn handle<'s>(
        &'s mut self,
        message: Message,
        acknowledger: MessageAcknowledger<'s, MTU>,
    ) -> Self::HandlerFut<'s> {
        async move {
            acknowledger.acknowledge().await;
            match message {
                Message::WriteFlash(_) => unimplemented!(),
                Message::ReadFlash(_) => unimplemented!(),
                Message::EraseFlash(_) => unimplemented!(),
            }
        }
    }
}
