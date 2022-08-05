use super::UsbBus;
use defmt::warn;
use embassy_nrf::usb::PowerUsb;
use embassy_usb::{control::OutResponse, driver::Driver, Builder};
use embassy_usb_hid::{HidReaderWriter, ReportId, RequestHandler, State};
use embassy_util::{
    blocking_mutex::raw::NoopRawMutex,
    channel::mpmc::{Channel, Receiver},
    Forever,
};
use engine::OutputCommand;
use futures::{future::join, sink, Sink};
use usbd_hid::descriptor::{KeyboardReport, SerializedDescriptor};

const POLL_INTERVAL_MS: u8 = 1;

static STATE: Forever<State> = Forever::new();
static CHANNEL: Forever<Channel<NoopRawMutex, Key, 1>> = Forever::new();
static REQUEST_HANDLER: GlobalRequestHandler = GlobalRequestHandler;

pub fn configure<D: Driver<'static>>(
    builder: &mut Builder<'static, D>,
) -> (Keyboard<'static>, KeyboardRuntime<D>) {
    let channel = CHANNEL.put(Channel::new());
    let state = STATE.put(State::new());

    let config = embassy_usb_hid::Config {
        report_descriptor: KeyboardReport::desc(),
        request_handler: Some(&REQUEST_HANDLER),
        poll_ms: POLL_INTERVAL_MS,
        max_packet_size: 64,
    };

    (
        Keyboard(channel),
        KeyboardRuntime {
            reader_writer: HidReaderWriter::<_, 1, 8>::new(builder, state, config),
            receiver: channel.receiver(),
        },
    )
}

#[embassy_executor::task]
pub async fn run(
    runtime: KeyboardRuntime<
        embassy_nrf::usb::Driver<'static, embassy_nrf::peripherals::USBD, PowerUsb>,
    >,
) {
    let (reader, mut writer) = runtime.reader_writer.split();

    let reader_fut = reader.run(false, &REQUEST_HANDLER);

    let writer_fut = async move {
        let mut active_modifiers: u8 = 0;
        let mut previous_key = None;

        loop {
            let reset_report = KeyboardReport {
                modifier: 0,
                reserved: 0,
                leds: 0,
                keycodes: [0; 6],
            };

            // Receive the next key or reset all modifiers/keys if there is none available
            let key = if let Ok(key) = runtime.receiver.try_recv() {
                if previous_key == Some(key) {
                    if let Err(e) = writer.write_serialize(&reset_report).await {
                        warn!("failed to send keyboard report: {:?}", e);
                    }
                }

                key
            } else {
                if let Err(e) = writer.write_serialize(&reset_report).await {
                    warn!("failed to send keyboard report: {:?}", e);
                }
                runtime.receiver.recv().await
            };

            previous_key = Some(key);

            let report = character_to_report(key);

            if UsbBus::is_suspended() {
                UsbBus::wake_up();
            } else {
                if report.modifier != active_modifiers {
                    let reset_report = KeyboardReport {
                        modifier: report.modifier,
                        reserved: 0,
                        leds: 0,
                        keycodes: [0; 6],
                    };

                    match writer.write_serialize(&reset_report).await {
                        Ok(_) => {
                            active_modifiers = report.modifier;
                        }
                        Err(e) => warn!("failed to send keyboard report: {:?}", e),
                    }
                }

                if let Err(e) = writer.write_serialize(&report).await {
                    warn!("failed to send keyboard report: {:?}", e);
                }
            }
        }
    };

    join(reader_fut, writer_fut).await;
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Key {
    Character(char),
    Escape,
    Backspace,
}

#[derive(Clone, Copy)]
enum SidedModifier {
    Left(Modifier),
    Right(Modifier),
}

#[derive(Clone, Copy)]
enum Modifier {
    Control,
    Shift,
    Alt,
    Meta,
}

impl From<SidedModifier> for u8 {
    fn from(side: SidedModifier) -> Self {
        match side {
            SidedModifier::Left(modifier) => modifier.into(),
            SidedModifier::Right(modifier) => u8::from(modifier) << 4,
        }
    }
}

impl From<Modifier> for u8 {
    fn from(modifier: Modifier) -> Self {
        match modifier {
            Modifier::Control => 1 << 0,
            Modifier::Shift => 1 << 1,
            Modifier::Alt => 1 << 2,
            Modifier::Meta => 1 << 3,
        }
    }
}

#[derive(Clone, Copy)]
pub struct Keyboard<'c>(&'c Channel<NoopRawMutex, Key, 1>);

impl<'c> Keyboard<'c> {
    pub async fn send(&self, character: Key) {
        self.0.send(character).await
    }

    pub async fn send_str(&self, string: impl AsRef<str>) {
        for c in string.as_ref().chars() {
            self.send(Key::Character(c)).await;
        }
    }

    pub fn into_sink(self) -> impl Sink<OutputCommand> + 'c {
        sink::unfold(self.0, |channel, command| async move {
            match command {
                OutputCommand::Write(character) => {
                    channel.send(Key::Character(character)).await;
                }
                OutputCommand::Backspace(count) => {
                    for _ in 0..count {
                        channel.send(Key::Backspace).await;
                    }
                }
            }

            Ok::<_, ()>(channel)
        })
    }
}

// Safe wrapper that restricts access to the reader/writer so that only a properly configured one can be used with the fns in this module
pub struct KeyboardRuntime<D: Driver<'static>> {
    reader_writer: HidReaderWriter<'static, D, 1, 8>,
    receiver: Receiver<'static, NoopRawMutex, Key, 1>,
}

struct GlobalRequestHandler;

impl RequestHandler for GlobalRequestHandler {
    fn get_report(&self, _id: ReportId, _buf: &mut [u8]) -> Option<usize> {
        // debug!("Get report for {:?}", id);
        None
    }

    fn set_report(&self, _id: ReportId, _data: &[u8]) -> OutResponse {
        // debug!("Set report for {:?}: {=[u8]}", id, data);
        OutResponse::Accepted
    }
}

fn character_to_report(key: Key) -> KeyboardReport {
    // Keycodes taken from here:
    //      https://gist.github.com/MightyPork/6da26e382a7ad91b5496ee55fdc73db2
    // Which in turn is derived from here (page 88):
    //      https://usb.org/sites/default/files/hut1_3_0.pdf
    let keycode: (u8, &'static [Modifier]) = match key {
        Key::Character(c) => match c {
            'a' => (0x04, &[]),
            'b' => (0x05, &[]),
            'c' => (0x06, &[]),
            'd' => (0x07, &[]),
            'e' => (0x08, &[]),
            'f' => (0x09, &[]),
            'g' => (0x0A, &[]),
            'h' => (0x0B, &[]),
            'i' => (0x0C, &[]),
            'j' => (0x0D, &[]),
            'k' => (0x0E, &[]),
            'l' => (0x0F, &[]),
            'm' => (0x10, &[]),
            'n' => (0x11, &[]),
            'o' => (0x12, &[]),
            'p' => (0x13, &[]),
            'q' => (0x14, &[]),
            'r' => (0x15, &[]),
            's' => (0x16, &[]),
            't' => (0x17, &[]),
            'u' => (0x18, &[]),
            'v' => (0x19, &[]),
            'w' => (0x1A, &[]),
            'x' => (0x1B, &[]),
            'y' => (0x1C, &[]),
            'z' => (0x1D, &[]),

            'A' => (0x04, &[Modifier::Shift]),
            'B' => (0x05, &[Modifier::Shift]),
            'C' => (0x06, &[Modifier::Shift]),
            'D' => (0x07, &[Modifier::Shift]),
            'E' => (0x08, &[Modifier::Shift]),
            'F' => (0x09, &[Modifier::Shift]),
            'G' => (0x0A, &[Modifier::Shift]),
            'H' => (0x0B, &[Modifier::Shift]),
            'I' => (0x0C, &[Modifier::Shift]),
            'J' => (0x0D, &[Modifier::Shift]),
            'K' => (0x0E, &[Modifier::Shift]),
            'L' => (0x0F, &[Modifier::Shift]),
            'M' => (0x10, &[Modifier::Shift]),
            'N' => (0x11, &[Modifier::Shift]),
            'O' => (0x12, &[Modifier::Shift]),
            'P' => (0x13, &[Modifier::Shift]),
            'Q' => (0x14, &[Modifier::Shift]),
            'R' => (0x15, &[Modifier::Shift]),
            'S' => (0x16, &[Modifier::Shift]),
            'T' => (0x17, &[Modifier::Shift]),
            'U' => (0x18, &[Modifier::Shift]),
            'V' => (0x19, &[Modifier::Shift]),
            'W' => (0x1A, &[Modifier::Shift]),
            'X' => (0x1B, &[Modifier::Shift]),
            'Y' => (0x1C, &[Modifier::Shift]),
            'Z' => (0x1D, &[Modifier::Shift]),

            '1' => (0x1E, &[]),
            '2' => (0x1F, &[]),
            '3' => (0x20, &[]),
            '4' => (0x21, &[]),
            '5' => (0x22, &[]),
            '6' => (0x23, &[]),
            '7' => (0x24, &[]),
            '8' => (0x25, &[]),
            '9' => (0x26, &[]),
            '0' => (0x27, &[]),

            '!' => (0x1E, &[Modifier::Shift]),
            '@' => (0x1F, &[Modifier::Shift]),
            '#' => (0x20, &[Modifier::Shift]),
            '$' => (0x21, &[Modifier::Shift]),
            '%' => (0x22, &[Modifier::Shift]),
            '^' => (0x23, &[Modifier::Shift]),
            '&' => (0x24, &[Modifier::Shift]),
            '*' => (0x25, &[Modifier::Shift]),
            '(' => (0x26, &[Modifier::Shift]),
            ')' => (0x27, &[Modifier::Shift]),

            '\n' => (0x28, &[]),
            '\t' => (0x2B, &[]),
            ' ' => (0x2C, &[]),
            '-' => (0x2D, &[]),
            '=' => (0x2E, &[]),
            '[' => (0x2F, &[]),
            ']' => (0x30, &[]),
            '\\' => (0x31, &[]),
            ';' => (0x33, &[]),
            '\'' => (0x34, &[]),
            '`' => (0x35, &[]),
            ',' => (0x36, &[]),
            '.' => (0x37, &[]),
            '/' => (0x38, &[]),

            '_' => (0x2D, &[Modifier::Shift]),
            '+' => (0x2E, &[Modifier::Shift]),
            '{' => (0x2F, &[Modifier::Shift]),
            '}' => (0x30, &[Modifier::Shift]),
            '|' => (0x31, &[Modifier::Shift]),
            ':' => (0x33, &[Modifier::Shift]),
            '"' => (0x34, &[Modifier::Shift]),
            '~' => (0x35, &[Modifier::Shift]),
            '<' => (0x36, &[Modifier::Shift]),
            '>' => (0x37, &[Modifier::Shift]),
            '?' => (0x38, &[Modifier::Shift]),
            _ => unimplemented!(),
        },
        Key::Escape => (0x29, &[]),
        Key::Backspace => (0x2A, &[]),
    };

    KeyboardReport {
        modifier: keycode.1.into_iter().fold(0, |acc, m| acc + u8::from(*m)),
        reserved: 0,
        leds: 0,
        keycodes: [keycode.0, 0, 0, 0, 0, 0],
    }
}
