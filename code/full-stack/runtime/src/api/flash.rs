use crate::message::flash::{
    EraseFlash, FlashContent, FlashErased, FlashWritten, ReadFlash, WriteFlash,
};
use cofit::{Handler, Host, Transmitter, Transport};
use core::future::Future;
use core::time::Duration;
use futures::StreamExt;
use futures::{channel::mpsc, SinkExt};
use std::collections::HashSet;
use std::sync::Arc;
use tokio::time::timeout;

const SECTOR_SIZE: u32 = 4096;

// Timeouts are per unit (sectors for erase, message for write/read)
const TIMEOUT_WRITE: Duration = Duration::from_millis(5);
const TIMEOUT_READ: Duration = Duration::from_millis(5);
const TIMEOUT_ERASE: Duration = Duration::from_secs(1);

enum FlashMessage {
    Content(FlashContent),
    Written(FlashWritten),
    Erased(FlashErased<63>),
}

pub enum FlashError {
    /// Data transmission was not acknowledged within time
    TimedOut,
}

pub struct FlashAPI<'t, T: Transport<63>> {
    tx: Arc<Transmitter<'static, 't, 63, T, Host>>,
    rx: mpsc::UnboundedReceiver<FlashMessage>,
}

impl<'t, T: Transport<63>> FlashAPI<'t, T> {
    pub(crate) fn new(
        tx: Arc<Transmitter<'static, 't, 63, T, Host>>,
    ) -> (Self, FlashReadHandler, FlashWriteHandler, FlashEraseHandler) {
        let (handler_tx, rx) = mpsc::unbounded();

        (
            Self { tx, rx },
            FlashReadHandler(handler_tx.clone()),
            FlashWriteHandler(handler_tx.clone()),
            FlashEraseHandler(handler_tx.clone()),
        )
    }

    pub async fn read(&mut self, offset: u32, bytes: &mut [u8]) -> Result<(), FlashError> {
        if offset % 4 != 0 {
            panic!("attempted to write unaligned data to flash")
        }

        let message: ReadFlash<63> = ReadFlash {
            start: offset.into(),
            end: (offset + bytes.len() as u32).into(),
        };

        self.clear_rx();
        self.tx.send(message).await;

        for (offset, chunk) in bytes
            .chunks_mut(60)
            .enumerate()
            .map(|(i, chunk)| (offset + 60 * i as u32, chunk))
        {
            let content_fut = async {
                while let Some(msg) = self.rx.next().await {
                    match msg {
                        FlashMessage::Content(content) if content.offset == offset.into() => {
                            chunk.copy_from_slice(&content.data[0..chunk.len()]);
                            break;
                        }
                        _ => {} // TODO Print a warning that we received an unexpected flash message
                    }
                }
            };

            _ = timeout(TIMEOUT_READ, content_fut).await;
        }

        Ok(())
    }

    pub async fn erase(&mut self, start: u32, end: u32) -> Result<(), FlashError> {
        if start % SECTOR_SIZE != 0 || end % SECTOR_SIZE != 0 || start >= end {
            panic!("attempted to erase non-aligned sectors")
        }

        let message: EraseFlash<63> = EraseFlash {
            start_sector: (start / SECTOR_SIZE) as u16,
            end_sector: (end / SECTOR_SIZE) as u16,
        };
        let sector_count = message.end_sector as u32 - message.start_sector as u32;

        self.clear_rx();
        self.tx.send(message).await;

        let ack_fut = async {
            while let Some(msg) = self.rx.next().await {
                match msg {
                    FlashMessage::Erased(ack) if ack == message.into() => break,
                    _ => {} // TODO Print a warning that we received an unexpected flash message
                }
            }
        };

        if timeout(TIMEOUT_ERASE * sector_count, ack_fut).await.is_ok() {
            Ok(())
        } else {
            Err(FlashError::TimedOut)
        }
    }

    pub async fn write(&mut self, offset: u32, data: &[u8]) -> Result<(), FlashError> {
        if offset % 4 != 0 || data.len() % 4 != 0 {
            panic!("attempted to write unaligned data to flash")
        }
        // Build a work queue
        let mut pending_writes: HashSet<WriteFlash> = data
            .chunks(60)
            .enumerate()
            .map(|(i, chunk)| {
                let offset = (offset + 60 * i as u32).into();
                let mut data = [1; 60];
                data[0..chunk.len()].copy_from_slice(chunk);

                WriteFlash { data, offset }
            })
            .collect();

        // Create a handler for ACKs
        let handle_ack = |pending_writes: &mut HashSet<WriteFlash>, msg: FlashMessage| match msg {
            FlashMessage::Written(ack) => {
                if !pending_writes.remove(&ack.into()) {
                    // TODO Print a warning that we received an ACK for something we did not write
                }
            }
            _ => {} // TODO Print a warning that we received an unexpected flash message
        };

        // Discard any pending messages
        self.clear_rx();

        // Try sending each item a couple of times
        let mut retry_limit = 3;
        while !pending_writes.is_empty() && retry_limit > 0 {
            // Send all remaining chunks
            for message in pending_writes.iter() {
                // TODO Progressively slow down with each while loop iteration
                self.tx.send(*message).await;
            }

            for _ in 0..pending_writes.len() {
                if let Ok(Some(msg)) = timeout(TIMEOUT_WRITE, self.rx.next()).await {
                    handle_ack(&mut pending_writes, msg);
                }
            }

            retry_limit -= 1;
        }

        if retry_limit == 0 {
            Err(FlashError::TimedOut)
        } else {
            Ok(())
        }
    }

    fn clear_rx(&mut self) {
        while let Ok(Some(_)) = self.rx.try_next() {}
    }
}

pub struct FlashReadHandler(mpsc::UnboundedSender<FlashMessage>);

impl Handler<63> for FlashReadHandler {
    type Message = FlashContent;

    type RecvFut<'s> = impl Future<Output = ()>
    where
        Self: 's;

    fn handle<'s>(&'s self, message: Self::Message) -> Self::RecvFut<'s> {
        async move {
            self.0
                .clone()
                .send(FlashMessage::Content(message))
                .await
                .ok();
        }
    }
}

pub struct FlashWriteHandler(mpsc::UnboundedSender<FlashMessage>);

impl Handler<63> for FlashWriteHandler {
    type Message = FlashWritten;

    type RecvFut<'s> = impl Future<Output = ()>
    where
        Self: 's;

    fn handle<'s>(&'s self, message: Self::Message) -> Self::RecvFut<'s> {
        async move {
            self.0
                .clone()
                .send(FlashMessage::Written(message))
                .await
                .ok();
        }
    }
}

pub struct FlashEraseHandler(mpsc::UnboundedSender<FlashMessage>);

impl Handler<63> for FlashEraseHandler {
    type Message = FlashErased<63>;

    type RecvFut<'s> = impl Future<Output = ()>
    where
        Self: 's;

    fn handle<'s>(&'s self, message: Self::Message) -> Self::RecvFut<'s> {
        async move {
            self.0
                .clone()
                .send(FlashMessage::Erased(message))
                .await
                .ok();
        }
    }
}
