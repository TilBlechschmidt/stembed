use super::UsbNetwork;
use crate::{
    cofit::{MessageAcknowledger, MessageHandler, SerializedMessage, Transport, WireFormat},
    firmware::{AlignedArray, FlashController},
};
use core::future::Future;
use embedded_storage_async::nor_flash::AsyncNorFlash;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct DataRange {
    pub offset: u64,
    pub length: u64,
}

#[derive(Serialize, Deserialize)]
pub enum Message {
    WriteFlash(DataRange),
    ReadFlash(DataRange),
    EraseFlash(DataRange),
    // TODO Add variant for sending error codes / messages over to the host
}

pub struct TestFormat;

impl<const MTU: usize> WireFormat<MTU> for TestFormat {
    type Message = Message;
    type Error = postcard::Error;

    fn serialize(&self, message: Self::Message) -> Result<SerializedMessage<MTU>, Self::Error> {
        let mut buf = [0; MTU];
        postcard::to_slice(&message, &mut buf)?;

        Ok(SerializedMessage {
            // For now we will just always use ID 42, in the future they will be dynamically negotiated between host and peripheral
            id: 42.into(),
            bytes: buf,
        })
    }

    fn deserialize(&self, packet: SerializedMessage<MTU>) -> Result<Self::Message, Self::Error> {
        if packet.id != 42.into() {
            return Err(postcard::Error::DeserializeBadEnum);
        }

        Ok(postcard::take_from_bytes(&packet.bytes)?.0)
    }
}

pub struct TestMessageHandler<'d, Flash: AsyncNorFlash, T: Transport<64>> {
    pub flash: &'d FlashController<Flash>,
    pub network: &'d UsbNetwork<'d, T, TestFormat>,
}

impl<'d, Flash: AsyncNorFlash, T: Transport<64>> TestMessageHandler<'d, Flash, T> {
    pub fn new(
        flash: &'d FlashController<Flash>,
        network: &'d UsbNetwork<'d, T, TestFormat>,
    ) -> Self {
        Self { flash, network }
    }

    async fn handle_flash_write<const MTU: usize>(
        &self,
        range: DataRange,
        acknowledger: MessageAcknowledger<'_, MTU>,
    ) {
        #[cfg(feature = "defmt")]
        defmt::debug!("writing flash range {} + {}", range.offset, range.length);

        // TODO Sanity-check the range and send an error code
        let mut reader = self.network.create_stream_reader().await;
        let mut offset = range.offset as u32;
        let end_offset = (range.offset + range.length) as u32;

        acknowledger.acknowledge().await;

        while let Some(data) = reader.recv().await {
            if offset >= end_offset {
                #[cfg(feature = "defmt")]
                defmt::error!("attempted to write data beyond the initially announced range");
                break;
            }

            // TODO Maintain a buffer so that all writes are aligned to and sized according to Flash::WRITE_SIZE
            // TODO Make this a feature of the flash controller! That way we can internally maintain caches to optimize things.
            if self.flash.write(offset, &data).await.is_err() {
                #[cfg(feature = "defmt")]
                defmt::error!("failed to write to flash at offset {}", offset);
            }
            offset += data.len() as u32;
        }
    }

    async fn handle_flash_read<const MTU: usize>(
        &self,
        range: DataRange,
        acknowledger: MessageAcknowledger<'_, MTU>,
    ) {
        #[cfg(feature = "defmt")]
        defmt::debug!("reading flash range {} + {}", range.offset, range.length);

        let mut writer = self
            .network
            .create_stream_writer(|read_offset| async move {
                if read_offset >= range.offset + range.length {
                    None
                } else {
                    let mut bytes = AlignedArray::<61>::new();
                    self.flash
                        .read_aligned((range.offset + read_offset) as u32, &mut (*bytes))
                        .await
                        .expect("failed to read flash");
                    Some(bytes.into())
                }
            })
            .await;

        acknowledger.acknowledge().await;

        while !writer.send().await {}
    }

    async fn handle_flash_erase<const MTU: usize>(
        &self,
        range: DataRange,
        acknowledger: MessageAcknowledger<'_, MTU>,
    ) {
        #[cfg(feature = "defmt")]
        defmt::debug!("erasing flash range {} + {}", range.offset, range.length);
        if let Err(_) = self
            .flash
            .erase_aligned_chunk(range.offset as u32, (range.offset + range.length) as u32)
            .await
        {
            #[cfg(feature = "defmt")]
            defmt::error!(
                "failed to erase flash in range {} + {}",
                range.offset,
                range.length
            );
        }

        acknowledger.acknowledge().await;
    }
}

impl<'d, Flash: AsyncNorFlash, T: Transport<64>, const MTU: usize> MessageHandler<Message, MTU>
    for TestMessageHandler<'d, Flash, T>
where
    Flash: 'd,
    T: 'd,
{
    type HandlerFut<'s> = impl Future<Output = ()> + 's
    where
        Self: 's;

    fn handle<'s>(
        &'s mut self,
        message: Message,
        acknowledger: MessageAcknowledger<'s, MTU>,
    ) -> Self::HandlerFut<'s> {
        async move {
            match message {
                Message::WriteFlash(range) => self.handle_flash_write(range, acknowledger).await,
                Message::ReadFlash(range) => self.handle_flash_read(range, acknowledger).await,
                Message::EraseFlash(range) => self.handle_flash_erase(range, acknowledger).await,
            }
        }
    }
}
