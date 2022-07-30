use core::future::Future;
use defmt::{debug, warn, Format};
use embassy::{
    blocking_mutex::raw::NoopRawMutex,
    channel::{
        self,
        mpmc::{Channel as EmbassyChannel, Receiver, Sender},
    },
    time::Duration,
    util::Forever,
};
use embassy_usb::{control::OutResponse, driver::Driver, Builder};
use embassy_usb_hid::{HidReaderWriter, ReportId, RequestHandler, State};
use futures::future::join;
use shittyruntime::cofit::Transport;
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

impl<'c> Transport<64> for Channel<'c> {
    type TxFut<'t> = impl Future<Output = ()> + 't where Self: 't;
    type RxFut<'t> = impl Future<Output = [u8; 64]> + 't where Self: 't;

    fn send<'t>(&'t self, data: [u8; 64]) -> Self::TxFut<'t> {
        self.tx.send(Packet(data))
    }

    fn recv<'t>(&'t self) -> Self::RxFut<'t> {
        async move { self.rx.recv().await.0 }
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

#[embassy::task]
pub async fn run(
    runtime: ChannelRuntime<
        'static,
        embassy_nrf::usb::Driver<'static, embassy_nrf::peripherals::USBD>,
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

        if let Err(e) = self.host_device_sender.try_send(command) {
            warn!("failed to forward host -> device command, lagging behind");
            OutResponse::Rejected
        } else {
            OutResponse::Accepted
        }
    }

    fn set_idle(&self, id: Option<ReportId>, dur: Duration) {
        debug!("Set idle rate for {:?} to {:?}", id, dur);
    }

    fn get_idle(&self, id: Option<ReportId>) -> Option<Duration> {
        debug!("Get idle rate for {:?}", id);
        None
    }
}
