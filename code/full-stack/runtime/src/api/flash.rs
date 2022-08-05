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
use std::time::Instant;
use tokio::time::timeout;

const CHUNK_SIZE: usize = 60;
const SECTOR_SIZE: u32 = 4096;

// Timeouts are per unit (sectors for erase, message for write/read)
const TIMEOUT_READ: Duration = Duration::from_millis(25);
const TIMEOUT_WRITE: Duration = Duration::from_millis(25);
const TIMEOUT_ERASE: Duration = Duration::from_secs(1);

const WRITE_INTERVAL: Duration = Duration::from_nanos(250000);

#[derive(Debug)]
enum FlashMessage {
    Content(FlashContent),
    Written(FlashWritten),
    Erased(FlashErased<63>),
}

#[derive(Debug)]
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

    // TODO Build a version of this which implements AsyncRead with a method to pre-fetch and alternatively have it continously re-issue read requests
    pub async fn read(&mut self, offset: u32, bytes: &mut [u8]) -> Result<(), FlashError> {
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
                        _ => {
                            // TODO Print a warning that we received an unexpected flash message
                            println!(
                                "received unexpected flash message (expected_offset={offset}): {:?}",
                                msg
                            );
                        }
                    }
                }
            };

            if timeout(TIMEOUT_READ, content_fut).await.is_err() {
                return Err(FlashError::TimedOut);
            }
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

    #[must_use]
    pub fn write<'s>(&'s mut self, offset: u32, data: &'s [u8]) -> FlashWriteTask<'s, 't, T> {
        FlashWriteTask::new(self, data, offset)

        // TODO Figure out the lifetimes for making it return a stream instead
        // stream::unfold(Some(task), |task| async move {
        //     let mut task = task?;
        //     match task.next().await {
        //         Ok(Some(progress)) => Some((Ok(progress), Some(task))),
        //         Ok(None) => None,
        //         Err(error) => Some((Err(error), None)),
        //     }
        // })
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

pub struct FlashWriteTask<'r, 't, T: Transport<63>> {
    flash: &'r mut FlashAPI<'t, T>,
    data: &'r [u8],
    offset: u32,
    count: u32,
    queue: HashSet<usize>,
}

impl<'r, 't, T: Transport<63>> FlashWriteTask<'r, 't, T> {
    fn new(flash: &'r mut FlashAPI<'t, T>, data: &'r [u8], offset: u32) -> Self {
        let chunk_count = data.chunks(CHUNK_SIZE).count();

        Self {
            flash,
            data,
            offset,
            count: chunk_count as u32,
            queue: (0..chunk_count).collect(),
        }
    }

    pub async fn next(&mut self) -> Result<Option<f64>, FlashError> {
        let remaining = self.queue.len();
        let next_fut = async {
            loop {
                let new_remaining = self.try_next().await?;
                if remaining != new_remaining {
                    return Some(1.0 - (new_remaining as f64 / self.count as f64));
                }
            }
        };

        timeout(TIMEOUT_WRITE, next_fut)
            .await
            .map_err(|_| FlashError::TimedOut)
    }

    async fn try_next(&mut self) -> Option<usize> {
        // Get the next item from the work queue
        let index: usize = *(self.queue.iter().next()?);
        self.queue.remove(&index);

        // Send the data
        let start = Instant::now();
        let message = self.message(index)?;
        self.flash.tx.send(message).await;

        // Look if we have some ACK waiting :)
        let _ = timeout(
            WRITE_INTERVAL - start.elapsed().max(WRITE_INTERVAL),
            self.wait_for_ack(),
        )
        .await;

        Some(self.queue.len())
    }

    async fn wait_for_ack(&mut self) {
        match self.flash.rx.next().await {
            Some(FlashMessage::Written(ack)) => {
                let index = self.index_from_offset(*ack.offset);
                if !(Some(ack.into()) == self.message(index) && self.queue.remove(&index)) {
                    // TODO Print a warning that we received an ACK for something we did not write
                }
            }
            _ => {} // TODO Print a warning that we received an unexpected flash message
        }
    }

    fn message(&self, index: usize) -> Option<WriteFlash> {
        let chunk = self
            .data
            .chunks(CHUNK_SIZE)
            .skip(index)
            .next()
            .expect("attempted to build chunk for non-existent index");
        let offset = (self.offset + (index * CHUNK_SIZE) as u32).into();

        let mut data = [255; CHUNK_SIZE];
        data[0..chunk.len()].copy_from_slice(chunk);

        Some(WriteFlash { data, offset })
    }

    fn index_from_offset(&self, offset: u32) -> usize {
        (offset - self.offset) as usize / CHUNK_SIZE
    }
}
