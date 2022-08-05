use cofit::Transport;
use core::future::Future;
use defmt::warn;
use embassy_nrf::usb::PowerUsb;
use embassy_usb::{control::OutResponse, driver::Driver, Builder};
use embassy_usb_hid::{HidReaderWriter, ReportId, RequestHandler, State};
use embassy_util::{
    blocking_mutex::raw::NoopRawMutex,
    channel::mpmc::{Channel as EmbassyChannel, Receiver, Sender},
    Forever,
};
use futures::future::join;
use usbd_hid::descriptor::{gen_hid_descriptor, SerializedDescriptor};

const POLL_INTERVAL_MS: u8 = 1;

const INCOMING_BUFFER_LEN: usize = 8;
const OUTGOING_BUFFER_LEN: usize = 8;

const INCOMING_USB_BUFFER_LEN: usize = 64;
const OUTGOING_USB_BUFFER_LEN: usize = 64;

static STATE: Forever<State> = Forever::new();
static HOST_DEVICE_CHANNEL: Forever<EmbassyChannel<NoopRawMutex, Packet, INCOMING_BUFFER_LEN>> =
    Forever::new();
static DEVICE_HOST_CHANNEL: Forever<EmbassyChannel<NoopRawMutex, Packet, OUTGOING_BUFFER_LEN>> =
    Forever::new();
static REQUEST_HANDLER: Forever<CommandChannelRequestHandler> = Forever::new();

pub struct Packet([u8; 64]);

pub struct Channel<'c> {
    tx: &'c EmbassyChannel<NoopRawMutex, Packet, OUTGOING_BUFFER_LEN>,
    rx: Receiver<'c, NoopRawMutex, Packet, INCOMING_BUFFER_LEN>,
}

pub struct ChannelRuntime<'r, D: Driver<'static>> {
    request_handler: &'r CommandChannelRequestHandler,
    reader_writer: HidReaderWriter<'static, D, OUTGOING_USB_BUFFER_LEN, INCOMING_USB_BUFFER_LEN>,
    device_host_receiver: Receiver<'static, NoopRawMutex, Packet, OUTGOING_BUFFER_LEN>,
}

impl<'c> Transport<63> for Channel<'c> {
    type TxFut<'t> = impl Future<Output = ()> + 't where Self: 't;
    type RxFut<'t> = impl Future<Output = (u8, [u8; 63])> + 't where Self: 't;

    fn send<'t>(&'t self, id: u8, data: [u8; 63]) -> Self::TxFut<'t> {
        let mut packet = [0; 64];
        packet[0] = id;
        packet[1..].copy_from_slice(&data);

        self.tx.send(Packet(packet))
    }

    fn recv<'t>(&'t self) -> Self::RxFut<'t> {
        async move {
            let packet = self.rx.recv().await.0;
            let mut data = [0; 63];
            data.copy_from_slice(&packet[1..]);
            (packet[0], data)
        }
    }
}

pub fn configure<D: Driver<'static>>(
    builder: &mut Builder<'static, D>,
) -> (Channel<'static>, ChannelRuntime<'static, D>) {
    let host_device_channel = HOST_DEVICE_CHANNEL.put(EmbassyChannel::new());
    let device_host_channel = DEVICE_HOST_CHANNEL.put(EmbassyChannel::new());
    let state = STATE.put(State::new());
    let request_handler = REQUEST_HANDLER.put(CommandChannelRequestHandler {
        host_device_sender: host_device_channel.sender(),
    });

    let config = embassy_usb_hid::Config {
        report_descriptor: BidirectionalReport::desc(),
        request_handler: Some(request_handler),
        poll_ms: POLL_INTERVAL_MS,
        max_packet_size: 64,
    };

    let reader_writer = HidReaderWriter::<_, OUTGOING_USB_BUFFER_LEN, INCOMING_USB_BUFFER_LEN>::new(
        builder, state, config,
    );

    let runtime = ChannelRuntime {
        request_handler,
        reader_writer,
        device_host_receiver: device_host_channel.receiver(),
    };

    let command_channel = Channel {
        tx: device_host_channel,
        rx: host_device_channel.receiver(),
    };

    (command_channel, runtime)
}

#[embassy_executor::task]
pub async fn run(
    runtime: ChannelRuntime<
        'static,
        embassy_nrf::usb::Driver<'static, embassy_nrf::peripherals::USBD, PowerUsb>,
    >,
) {
    let (reader, mut writer) = runtime.reader_writer.split();

    let reader_fut = reader.run(false, runtime.request_handler);
    let writer_fut = async move {
        loop {
            let packet = runtime.device_host_receiver.recv().await;

            if let Err(e) = writer.write(&packet.0).await {
                warn!("failed to send command: {:?}", e);
            }
        }
    };

    join(reader_fut, writer_fut).await;
}

#[gen_hid_descriptor(
    (report_id = 0x0, collection = APPLICATION, usage_page = VENDOR_DEFINED_START, usage = 0x42) = {
        input=input;
        output=output;
    }
)]
struct BidirectionalReport {
    input: [u8; 64],
    output: [u8; 64],
}

struct CommandChannelRequestHandler {
    host_device_sender: Sender<'static, NoopRawMutex, Packet, INCOMING_BUFFER_LEN>,
}

impl RequestHandler for CommandChannelRequestHandler {
    fn get_report(&self, _id: ReportId, _buf: &mut [u8]) -> Option<usize> {
        None
    }

    fn set_report(&self, id: ReportId, data: &[u8]) -> OutResponse {
        if id != ReportId::Out(0) || data.len() != 64 {
            return OutResponse::Rejected;
        }

        let mut payload = [0; 64];
        payload.copy_from_slice(&data);

        let command = Packet(payload);

        if let Err(_) = self.host_device_sender.try_send(command) {
            warn!("failed to forward host -> device command, lagging behind");
            OutResponse::Rejected
        } else {
            OutResponse::Accepted
        }
    }
}
