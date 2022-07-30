use crate::{
    cofit::{Transport, UsbNetwork},
    firmware::{executor_support::*, AsyncOutputCommand, FlashController, Mpsc as _, Peripherals},
    input::InputState,
};
use embedded_storage_async::nor_flash::AsyncNorFlash;
use futures::{Sink, Stream};

pub mod messaging;
use messaging::*;

pub struct Runtime;

impl Runtime {
    pub async fn execute<
        I: Stream<Item = InputState>,
        C: Transport<64>,
        F: AsyncNorFlash,
        O: Sink<AsyncOutputCommand>,
    >(
        peripherals: Peripherals<I, C, F, O>,
    ) {
        let flash = FlashController::new(peripherals.flash);

        let ack_channel = Channel::new();
        let stream_channel = Channel::new();
        let message_channel = Channel::new();

        let network = UsbNetwork::new(
            peripherals.usb_channel,
            TestFormat,
            ack_channel.split(),
            stream_channel.split(),
            message_channel.split(),
        );

        let network_task = network.recv_task();

        let handler = TestMessageHandler::new(&flash, &network);
        let network_recv_task = network.recv_with(handler);

        // TODO Do more stuff like running the engine w/ select
        futures::join!(network_task, network_recv_task);
    }
}
