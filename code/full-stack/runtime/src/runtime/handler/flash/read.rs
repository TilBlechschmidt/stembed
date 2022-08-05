use super::super::super::Mutex;
use crate::message::flash::{FlashContent, ReadFlash};
use cofit::{Handler, Peripheral, Transmitter, Transport};
use core::future::Future;
use embedded_storage_async::nor_flash::AsyncNorFlash;

pub struct FlashReadHandler<'f, 't, F: AsyncNorFlash, T: Transport<63>> {
    flash: &'f Mutex<F>,
    tx: &'t Transmitter<'t, 't, 63, T, Peripheral>,
}

impl<'f, 't, F: AsyncNorFlash, T: Transport<63>> FlashReadHandler<'f, 't, F, T> {
    pub fn new(flash: &'f Mutex<F>, tx: &'t Transmitter<'t, 't, 63, T, Peripheral>) -> Self {
        Self { flash, tx }
    }
}

impl<'f, 't, F: AsyncNorFlash, T: Transport<63>> Handler<63> for FlashReadHandler<'f, 't, F, T> {
    type Message = ReadFlash<63>;

    type RecvFut<'s> = impl Future<Output = ()> + 's
    where
        Self: 's;

    fn handle<'s>(&'s self, message: Self::Message) -> Self::RecvFut<'s> {
        async move {
            let mut offset = *message.start;
            while offset < *message.end {
                let mut content = FlashContent {
                    data: [0; 60],
                    offset: offset.into(),
                };
                match self
                    .flash
                    .lock()
                    .await
                    .read(offset, &mut content.data)
                    .await
                {
                    Ok(_) => {
                        self.tx.send(content).await;
                    }
                    Err(_) => {
                        // TODO Print a warning that the read failed, maybe send a error message
                        break;
                    }
                }

                offset += 60;
            }
        }
    }
}
