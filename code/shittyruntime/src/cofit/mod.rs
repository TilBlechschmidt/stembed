//! Communication over fixed interval transports (like USB HID)

use self::stream::{StreamReadHandle, StreamWriteHandle};
use crate::firmware::{
    executor_support::{Channel, Mutex},
    Mpsc, MpscReceiver, MpscSender, Mutex as MutexTrait,
};
use core::future::Future;

const ACK_TIMEOUT_MS: u32 = 10_000; // 500;
const STREAM_RECV_TIMEOUT_MS: u32 = 10_000;

mod header;
mod stream;

pub use header::*;
pub use stream::StreamPacket;

pub trait Transport<const MTU: usize> {
    type TxFut<'t>: Future<Output = ()> + 't
    where
        Self: 't;

    type RxFut<'t>: Future<Output = [u8; MTU]> + 't
    where
        Self: 't;

    fn send<'t>(&'t self, data: [u8; MTU]) -> Self::TxFut<'t>;
    fn recv<'t>(&'t self) -> Self::RxFut<'t>;
}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub struct SerializedMessage<const MTU: usize> {
    pub id: ID,
    pub bytes: [u8; MTU],
}

pub trait WireFormat<const MTU: usize> {
    type Message;
    type Error;

    fn serialize(&self, message: Self::Message) -> Result<SerializedMessage<MTU>, Self::Error>;
    fn deserialize(&self, packet: SerializedMessage<MTU>) -> Result<Self::Message, Self::Error>;
}

pub struct MessageAcknowledger<'n, const MTU: usize> {
    serialized: SerializedMessage<MTU>,
    sender: <Channel<SerializedMessage<MTU>> as Mpsc>::Sender<'n>,
    sent: bool,
}

impl<'n, const MTU: usize> MessageAcknowledger<'n, MTU> {
    pub async fn acknowledge(mut self) {
        self.sender.send(self.serialized).await;
        self.sent = true;
    }
}

impl<const MTU: usize> Drop for MessageAcknowledger<'_, MTU> {
    fn drop(&mut self) {
        if !self.sent {
            panic!("Dropped MessageAcknowledger without acknowledging message");
        }
    }
}

pub trait MessageHandler<M, const MTU: usize> {
    type HandlerFut<'s>: Future<Output = ()> + 's
    where
        Self: 's;

    fn handle<'s>(
        &'s mut self,
        message: M,
        acknowledger: MessageAcknowledger<'s, MTU>,
    ) -> Self::HandlerFut<'s>;
}

#[derive(Debug)]
pub enum NetworkError<E> {
    FormatError(E),
    InvalidPacketHeader(PacketHeaderParseError),
    TimedOut,
    UnexpectedAck,
}

type MessageSender<'c, const PMTU: usize> = <Channel<SerializedMessage<PMTU>> as Mpsc>::Sender<'c>;
type MessageReceiver<'c, const PMTU: usize> =
    <Channel<SerializedMessage<PMTU>> as Mpsc>::Receiver<'c>;
type AckSender<'c, const PMTU: usize> = <Channel<SerializedMessage<PMTU>> as Mpsc>::Sender<'c>;
type AckReceiver<'c, const PMTU: usize> = <Channel<SerializedMessage<PMTU>> as Mpsc>::Receiver<'c>;
type StreamSender<'c, const PMTU: usize> = <Channel<StreamPacket<PMTU>> as Mpsc>::Sender<'c>;
type StreamReceiver<'c, const PMTU: usize> = <Channel<StreamPacket<PMTU>> as Mpsc>::Receiver<'c>;
type StreamReceiverLock<'t, const PMTU: usize> =
    <Mutex<StreamReceiver<'t, PMTU>> as MutexTrait>::Guard<'t>;

/// Variant of `Network` with MTUs for USB HID RAW transfer
pub type UsbNetwork<'c, T, F> = Network<'c, 64, 63, 61, T, F>;

// TODO PMTU & SMTU are constants derived from the TMTU. Somehow make them actually derive instead of having the user "guess" them
// TODO Make sure there can only ever be one message in-flight, because otherwise stuff will dead-lock :(
//      Unless we off-load stream processing into its own (semi-global) task per stream type, sending a message while a stream is being processed will break everything :D
//      A possibility would be to lock the send fn until no stream is active and no other message is in-flight.
//      => Actually, in that case, since we are not processing stuff in parallel anymore anyways, we can just go to a simple req/ack and req/res model not requiring the stream stuff anymore ...
pub struct Network<
    'c,
    const TMTU: usize,
    const PMTU: usize,
    const SMTU: usize,
    T: Transport<TMTU>,
    F: WireFormat<PMTU>,
> {
    transport: T,
    format: F,

    ack_sender: AckSender<'c, PMTU>,
    ack_receiver: Mutex<AckReceiver<'c, PMTU>>,

    stream_sender: StreamSender<'c, PMTU>,
    stream_receiver: Mutex<StreamReceiver<'c, PMTU>>,

    message_sender: MessageSender<'c, PMTU>,
    message_receiver: Mutex<MessageReceiver<'c, PMTU>>,
}

impl<
        'c,
        const TMTU: usize,
        const PMTU: usize,
        const SMTU: usize,
        T: Transport<TMTU> + 'c,
        F: WireFormat<PMTU> + 'c,
    > Network<'c, TMTU, PMTU, SMTU, T, F>
{
    pub fn new(
        transport: T,
        format: F,
        ack_channel: (AckSender<'c, PMTU>, AckReceiver<'c, PMTU>),
        stream_channel: (StreamSender<'c, PMTU>, StreamReceiver<'c, PMTU>),
        message_channel: (MessageSender<'c, PMTU>, MessageReceiver<'c, PMTU>),
    ) -> Self {
        assert_eq!(
            TMTU - 1,
            PMTU,
            "protocol MTU has to match transport MTU minus one"
        );
        assert_eq!(
            PMTU - 2,
            SMTU,
            "stream MTU has to match protocol MTU minus two / transport MTU minus three"
        );

        let (ack_sender, ack_receiver) = ack_channel;
        let ack_receiver = Mutex::new(ack_receiver);

        let (stream_sender, stream_receiver) = stream_channel;
        let stream_receiver = Mutex::new(stream_receiver);

        let (message_sender, message_receiver) = message_channel;
        let message_receiver = Mutex::new(message_receiver);

        Self {
            transport,
            format,
            ack_sender,
            ack_receiver,
            stream_sender,
            stream_receiver,
            message_sender,
            message_receiver,
        }
    }

    pub async fn create_stream_writer<
        'm,
        DataFut: Future<Output = Option<[u8; SMTU]>>,
        DataSource: FnMut(u64) -> DataFut,
    >(
        &'m self,
        data_source: DataSource,
    ) -> StreamWriteHandle<'m, TMTU, PMTU, SMTU, T, DataFut, DataSource>
    where
        'm: 'c,
    {
        // TODO Clear any left-over messages in the receiver
        StreamWriteHandle::new(
            self.stream_receiver.lock().await,
            &self.transport,
            data_source,
        )
    }

    pub async fn create_stream_reader<'m>(&'m self) -> StreamReadHandle<'m, TMTU, PMTU, SMTU, T>
    where
        'm: 'c,
    {
        // TODO Clear any left-over messages in the receiver
        StreamReadHandle::new(self.stream_receiver.lock().await, &self.transport)
    }

    pub async fn send(&self, message: F::Message) -> Result<(), NetworkError<F::Error>> {
        let serialized = self
            .format
            .serialize(message)
            .map_err(NetworkError::FormatError)?;
        let header = PacketHeader::Message(serialized.id);

        let mut data = [0; TMTU];
        data[0] = header.into();
        data[1..].copy_from_slice(&serialized.bytes);

        // Get hold of the acknowledgement mutex (there may only ever be one non-acked message in-flight)
        let mut ack_receiver = self.ack_receiver.lock().await;

        // Remove any pending acknowledgements
        let dropped_ack_count = ack_receiver.clear();
        if dropped_ack_count > 0 {
            #[cfg(feature = "defmt")]
            defmt::warn!("Dropped {} unexpected acknowledgements", dropped_ack_count);
        }

        // Send the actual data
        self.transport.send(data).await;

        // Wait for and verify the ACK
        if let Some(acknowledgement) = ack_receiver.recv_timeout(ACK_TIMEOUT_MS).await {
            if acknowledgement == serialized {
                Ok(())
            } else {
                Err(NetworkError::UnexpectedAck)
            }
        } else {
            Err(NetworkError::TimedOut)
        }
    }

    // Network task that processes incoming messages — has to be polled continously in the background for other functions to operate correctly
    pub async fn recv_task(&self) {
        loop {
            let data = self.transport.recv().await;
            let header: Result<PacketHeader, _> = data[0].try_into();

            match header {
                Ok(PacketHeader::Message(id)) => {
                    let mut serialized = SerializedMessage {
                        id,
                        bytes: [0; PMTU],
                    };

                    serialized.bytes.copy_from_slice(&data[1..]);

                    self.message_sender.send(serialized).await;
                }
                Ok(PacketHeader::MessageAck(id)) => {
                    let mut bytes = [0; PMTU];
                    bytes.copy_from_slice(&data[1..]);

                    self.ack_sender.send(SerializedMessage { id, bytes }).await;
                }
                Ok(PacketHeader::StreamPacket(header)) => {
                    // Check if someone has the stream receiver locked / a stream is open
                    if self.stream_receiver.try_lock().is_some() {
                        #[cfg(feature = "defmt")]
                        defmt::warn!("received stream packet while no stream is open");
                    }

                    // Forward the stream packet
                    let mut bytes = [0; PMTU];
                    bytes.copy_from_slice(&data[1..]);

                    let packet = StreamPacket { header, bytes };
                    self.stream_sender.send(packet).await;
                }
                Err(_) => {
                    #[cfg(feature = "defmt")]
                    defmt::debug!("received packet with invalid header");
                }
            }
        }
    }

    /// Receives incoming messages and processes them using the given message handler
    pub async fn recv_with<H: MessageHandler<F::Message, PMTU>>(&self, mut handler: H) {
        let mut receiver = self
            .message_receiver
            .try_lock()
            .expect("unable to lock message receiver, did you call recv_with twice?");

        let send_ack_channel = Channel::new();
        let mut send_ack_rx = send_ack_channel.receiver();

        let recv_task = async {
            loop {
                match receiver
                    .recv_timeout(u32::MAX)
                    .await
                    .map(|serialized| (serialized, self.format.deserialize(serialized)))
                {
                    Some((serialized, Ok(message))) => {
                        let acknowledger = MessageAcknowledger {
                            serialized,
                            sender: send_ack_channel.sender(),
                            sent: false,
                        };

                        handler.handle(message, acknowledger).await
                    }
                    Some((_, Err(_))) => {
                        #[cfg(feature = "defmt")]
                        defmt::error!("Failed to deserialize message");
                    }
                    None => {}
                }
            }
        };

        let ack_task = async {
            loop {
                if let Some(serialized) = send_ack_rx.recv_timeout(u32::MAX).await {
                    self.send_ack(serialized).await;
                }
            }
        };

        futures::join!(recv_task, ack_task);
    }

    async fn send_ack(&self, serialized: SerializedMessage<PMTU>) {
        let header = PacketHeader::MessageAck(serialized.id);

        let mut data = [0; TMTU];
        data[0] = header.into();
        data[1..].copy_from_slice(&serialized.bytes);

        self.transport.send(data).await
    }
}
