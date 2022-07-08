use defmt::{debug, warn, Format};
use embassy::{
    blocking_mutex::raw::NoopRawMutex,
    channel::mpmc::{Channel, Receiver, Sender},
    time::Duration,
    util::Forever,
};
use embassy_usb::{control::OutResponse, driver::Driver, Builder};
use embassy_usb_hid::{HidReaderWriter, ReportId, RequestHandler, State};
use futures::future::join;
use usbd_hid::descriptor::{gen_hid_descriptor, SerializedDescriptor};

const POLL_INTERVAL_MS: u8 = 1;

const INCOMING_BUFFER_LEN: usize = 8;
const OUTGOING_BUFFER_LEN: usize = 8;

const INCOMING_USB_BUFFER_LEN: usize = 64;
const OUTGOING_USB_BUFFER_LEN: usize = 64;

static STATE: Forever<State> = Forever::new();
static HOST_DEVICE_CHANNEL: Forever<Channel<NoopRawMutex, EncodedCommand, INCOMING_BUFFER_LEN>> =
    Forever::new();
static DEVICE_HOST_CHANNEL: Forever<Channel<NoopRawMutex, EncodedCommand, OUTGOING_BUFFER_LEN>> =
    Forever::new();
static REQUEST_HANDLER: Forever<CommandChannelRequestHandler> = Forever::new();

#[derive(Format)]
pub struct EncodedCommand {
    pub identifier: u8,
    pub payload: [u8; 63],
}

/// Sends commands to the USB task which forwards them to the host
#[derive(Clone, Copy)]
pub struct CommandSender<'c>(&'c Channel<NoopRawMutex, EncodedCommand, OUTGOING_BUFFER_LEN>);

impl<'c> CommandSender<'c> {
    pub async fn send(&self, command: EncodedCommand) {
        self.0.send(command).await;
    }
}

/// Receives commands from the host which have been forwarded by the USB task
pub struct CommandReceiver<'c>(Receiver<'c, NoopRawMutex, EncodedCommand, INCOMING_BUFFER_LEN>);

impl<'c> CommandReceiver<'c> {
    pub async fn recv(&self) -> EncodedCommand {
        self.0.recv().await
    }
}

pub struct CommandRuntime<'r, D: Driver<'static>> {
    request_handler: &'r CommandChannelRequestHandler,
    reader_writer: HidReaderWriter<'static, D, OUTGOING_USB_BUFFER_LEN, INCOMING_USB_BUFFER_LEN>,
    device_host_receiver: Receiver<'static, NoopRawMutex, EncodedCommand, OUTGOING_BUFFER_LEN>,
}

pub fn configure<D: Driver<'static>>(
    builder: &mut Builder<'static, D>,
) -> (
    CommandSender<'static>,
    CommandReceiver<'static>,
    CommandRuntime<'static, D>,
) {
    let host_device_channel = HOST_DEVICE_CHANNEL.put(Channel::new());
    let device_host_channel = DEVICE_HOST_CHANNEL.put(Channel::new());
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

    let runtime = CommandRuntime {
        request_handler,
        reader_writer,
        device_host_receiver: device_host_channel.receiver(),
    };

    let command_sender = CommandSender(device_host_channel);
    let command_receiver = CommandReceiver(host_device_channel.receiver());

    (command_sender, command_receiver, runtime)
}

#[embassy::task]
pub async fn run(
    runtime: CommandRuntime<
        'static,
        embassy_nrf::usb::Driver<'static, embassy_nrf::peripherals::USBD>,
    >,
) {
    let (reader, mut writer) = runtime.reader_writer.split();

    let reader_fut = reader.run(false, runtime.request_handler);
    let writer_fut = async move {
        loop {
            let command = runtime.device_host_receiver.recv().await;

            let mut data = [0; 64];
            data[0] = command.identifier;
            data[1..].copy_from_slice(&command.payload);

            if let Err(e) = writer.write(&data).await {
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
    host_device_sender: Sender<'static, NoopRawMutex, EncodedCommand, INCOMING_BUFFER_LEN>,
}

impl RequestHandler for CommandChannelRequestHandler {
    fn get_report(&self, _id: ReportId, _buf: &mut [u8]) -> Option<usize> {
        None
    }

    fn set_report(&self, id: ReportId, data: &[u8]) -> OutResponse {
        if id != ReportId::Out(0) || data.len() != 64 {
            return OutResponse::Rejected;
        }

        let mut payload = [0; 63];
        payload.copy_from_slice(&data[1..]);

        let command = EncodedCommand {
            identifier: data[0],
            payload,
        };

        if let Err(e) = self.host_device_sender.try_send(command) {
            warn!("failed to forward host->device command: {:?}", e);
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
