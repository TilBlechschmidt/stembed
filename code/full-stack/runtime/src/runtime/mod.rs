use self::{
    handler::{
        flash::{FlashEraseHandler, FlashReadHandler, FlashWriteHandler},
        IndirectHandler,
    },
    mutex::Mutex,
};
use super::message::flash::{
    EraseFlash, FlashContent, FlashErased, FlashWritten, ReadFlash, WriteFlash,
};
use cofit::{make_network, make_receiver_task, Peripheral, Transport};
use embedded_storage_async::nor_flash::AsyncNorFlash;
use engine::{InputState, OutputCommand};
use futures::{future::select, pin_mut, Sink, Stream};

mod handler;
mod hardware;
mod mutex;

pub use hardware::HardwareStack;

#[doc(cfg(feature = "runtime"))]
pub struct Runtime;

impl Runtime {
    pub async fn execute<
        I: Stream<Item = InputState>,
        C: Transport<63>,
        F: AsyncNorFlash,
        O: Sink<OutputCommand>,
    >(
        hardware: HardwareStack<I, C, F, O>,
    ) {
        // Initialize the network stack
        let (usb_tx, usb_rx) = make_network! {
            role:       Peripheral,
            transport:  &hardware.usb_channel,
            messages:   [
                ReadFlash<63>, FlashContent,
                WriteFlash, FlashWritten,
                EraseFlash<63>, FlashErased<63>
            ]
        };

        // Build the flash API
        let flash = Mutex::new(hardware.flash);

        //  ReadFlash
        let flash_read_handler = IndirectHandler::new(FlashReadHandler::new(&flash, &usb_tx));
        let flash_read_task = flash_read_handler.task();
        pin_mut!(flash_read_task);

        //  WriteFlash
        let flash_write_handler = IndirectHandler::new(FlashWriteHandler::new(&flash, &usb_tx));
        let flash_write_task = flash_write_handler.task();
        pin_mut!(flash_write_task);

        //  EraseFlash
        let flash_erase_handler = IndirectHandler::new(FlashEraseHandler::new(&flash, &usb_tx));
        let flash_erase_task = flash_erase_handler.task();
        pin_mut!(flash_erase_task);

        let flash_task = select(flash_read_task, select(flash_write_task, flash_erase_task));

        // Build the network task
        let usb_rx_task = make_receiver_task!(usb_rx, [flash_read_handler, flash_write_handler]);
        pin_mut!(usb_rx_task);

        // Run the runtime :)
        select(usb_rx_task, flash_task).await;
    }
}
