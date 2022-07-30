use super::super::super::Mutex;
use crate::message::flash::{FlashWritten, WriteFlash};
use cofit::{Handler, Peripheral, Transmitter, Transport};
use core::future::Future;
use embedded_storage_async::nor_flash::AsyncNorFlash;

pub struct FlashWriteHandler<'f, 't, F: AsyncNorFlash, T: Transport<63>> {
    flash: &'f Mutex<F>,
    tx: &'t Transmitter<'t, 't, 63, T, Peripheral>,
}

impl<'f, 't, F: AsyncNorFlash, T: Transport<63>> FlashWriteHandler<'f, 't, F, T> {
    pub fn new(flash: &'f Mutex<F>, tx: &'t Transmitter<'t, 't, 63, T, Peripheral>) -> Self {
        Self { flash, tx }
    }
}

impl<'f, 't, F: AsyncNorFlash, T: Transport<63>> Handler<63> for FlashWriteHandler<'f, 't, F, T> {
    type Message = WriteFlash;

    type RecvFut<'s> = impl Future<Output = ()> + 's
    where
        Self: 's;

    fn handle<'s>(&'s self, message: Self::Message) -> Self::RecvFut<'s> {
        async move {
            if *message.offset % 4 != 0 {
                // TODO Print a warning that someone attempted unaligned flash writes.
                //      Maybe even send a error message back? Might be sufficient to have it as log output as this counts as "API abuse"
                return;
            }

            let result = self
                .flash
                .lock()
                .await
                .write(message.offset.into(), &message.data)
                .await;

            match result {
                Ok(_) => {
                    let acknowledgement: FlashWritten = message.into();
                    self.tx.send(acknowledgement).await;
                }
                Err(_) => {
                    // TODO Print a warning!
                }
            }
        }
    }
}
