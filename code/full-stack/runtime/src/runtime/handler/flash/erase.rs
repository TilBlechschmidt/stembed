use super::super::super::Mutex;
use crate::message::flash::{EraseFlash, FlashErased};
use cofit::{Handler, Peripheral, Transmitter, Transport};
use core::future::Future;
use embedded_storage_async::nor_flash::AsyncNorFlash;

pub struct FlashEraseHandler<'f, 't, F: AsyncNorFlash, T: Transport<63>> {
    flash: &'f Mutex<F>,
    tx: &'t Transmitter<'t, 't, 63, T, Peripheral>,
}

impl<'f, 't, F: AsyncNorFlash, T: Transport<63>> FlashEraseHandler<'f, 't, F, T> {
    pub fn new(flash: &'f Mutex<F>, tx: &'t Transmitter<'t, 't, 63, T, Peripheral>) -> Self {
        Self { flash, tx }
    }
}

impl<'f, 't, F: AsyncNorFlash, T: Transport<63>> Handler<63> for FlashEraseHandler<'f, 't, F, T> {
    type Message = EraseFlash<63>;

    type RecvFut<'s> = impl Future<Output = ()> + 's
    where
        Self: 's;

    fn handle<'s>(&'s self, message: Self::Message) -> Self::RecvFut<'s> {
        async move {
            let start = (message.start_sector as usize * F::ERASE_SIZE) as u32;
            let end = (message.end_sector as usize * F::ERASE_SIZE) as u32;

            let result = self.flash.lock().await.erase(start, end).await;

            match result {
                Ok(_) => {
                    let acknowledgement: FlashErased<63> = message.into();
                    self.tx.send(acknowledgement).await;
                }
                Err(_) => {
                    // TODO Print a warning!
                }
            }
        }
    }
}
