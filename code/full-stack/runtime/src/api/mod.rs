use super::message::flash::{
    EraseFlash, FlashContent, FlashErased, FlashWritten, ReadFlash, WriteFlash,
};
use cofit::{make_network, make_owned_receiver_task, Host, Transmitter, Transport};
use core::{future::Future, ops::DerefMut};
use futures::lock::Mutex;
use std::sync::Arc;

mod flash;

pub use flash::FlashAPI;

#[derive(Clone)]
pub struct RuntimeAPI<'t, T: Transport<63>> {
    tx: Arc<Transmitter<'static, 't, 63, T, Host>>,
    flash: Arc<Mutex<FlashAPI<'t, T>>>,
}

impl<'t, T: Transport<63>> RuntimeAPI<'t, T> {
    pub fn new(transport: &'t T) -> (impl Future + 't, Self) {
        let (tx, rx) = make_network! {
            role:       Host,
            transport:  transport,
            messages:   [
                ReadFlash<63>, FlashContent,
                WriteFlash, FlashWritten,
                EraseFlash<63>, FlashErased<63>
            ]
        };

        let tx = Arc::new(tx);

        let (flash, flash_read_handler, flash_write_handler, flash_erase_handler) =
            flash::FlashAPI::new(tx.clone());

        let flash = Arc::new(Mutex::new(flash));

        let rx_task = make_owned_receiver_task!(
            rx,
            [flash_read_handler, flash_write_handler, flash_erase_handler]
        );

        (rx_task, Self { tx, flash })
    }

    pub async fn reset(&self) {
        self.tx.reset_peripheral().await;
    }

    /// Acquires a mutable handle to the flash API
    pub async fn flash(&self) -> impl DerefMut<Target = FlashAPI<'t, T>> + '_ {
        self.flash.lock().await
    }
}
