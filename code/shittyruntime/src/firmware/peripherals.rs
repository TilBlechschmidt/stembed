use crate::{cofit::Transport, input::InputState};
use embedded_storage_async::nor_flash::AsyncNorFlash;
use futures::{Sink, Stream};

// TODO: Make the regular output command contain a single char only and map it at some point.
//       This way it is way simpler to deal with the output and the iteration is handled elsewhere.
//       Besides, this type should be part of the engine crate ;)
pub enum AsyncOutputCommand {
    Write(char),
    Backspace(u8),
}

pub struct Peripherals<
    I: Stream<Item = InputState>,
    C: Transport<64>,
    F: AsyncNorFlash,
    O: Sink<AsyncOutputCommand>,
> {
    pub input: I,
    pub usb_output: O,
    pub usb_channel: C,
    pub flash: F,
}
