use cofit::Transport;
use embedded_storage_async::nor_flash::AsyncNorFlash;
use engine::{InputState, OutputCommand};
use futures::{Sink, Stream};

/// Set of hardware interface implementations
#[doc(cfg(feature = "runtime"))]
pub struct HardwareStack<
    I: Stream<Item = InputState>,
    C: Transport<63>,
    F: AsyncNorFlash,
    O: Sink<OutputCommand>,
> {
    pub input: I,
    pub usb_output: O,
    pub usb_channel: C,
    pub flash: F,
}
