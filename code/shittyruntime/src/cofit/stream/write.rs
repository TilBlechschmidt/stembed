use super::{
    super::StreamReceiverLock, MpscReceiver, StreamClosePacket, StreamPacketHeader,
    StreamRevertPacket, StreamSequenceID, Transport,
};
use crate::cofit::{PacketHeader, STREAM_RECV_TIMEOUT_MS};
use core::future::Future;
use PacketHeader::*;
use StreamPacketHeader::*;

#[derive(PartialEq, Eq)]
enum StreamState {
    Transmitting,
    ReachedEnd,
    Closed,
}

pub struct StreamWriteHandle<
    't,
    const TMTU: usize,
    const PMTU: usize,
    const SMTU: usize,
    T: Transport<TMTU>,
    F: Future<Output = Option<[u8; SMTU]>>,
    D: FnMut(u64) -> F,
> {
    receiver: StreamReceiverLock<'t, PMTU>,
    transport: &'t T,

    data_source: D,

    sequence_id: StreamSequenceID,
    state: StreamState,
}

impl<
        't,
        const TMTU: usize,
        const PMTU: usize,
        const SMTU: usize,
        T: Transport<TMTU>,
        F: Future<Output = Option<[u8; SMTU]>>,
        D: FnMut(u64) -> F,
    > StreamWriteHandle<'t, TMTU, PMTU, SMTU, T, F, D>
{
    pub fn new(receiver: StreamReceiverLock<'t, PMTU>, transport: &'t T, data_source: D) -> Self {
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

        Self {
            receiver,
            transport,
            data_source,
            sequence_id: StreamSequenceID(0),
            state: StreamState::Transmitting,
        }
    }

    /// Operates the stream and continually transmits data until everything has been transmitted at which point `true` is returned.
    /// Calling this method after it returned true once will panic.
    // TODO Fuse this method so it either returns self or nothing upon completion.
    pub async fn send(&mut self) -> bool {
        if self.state == StreamState::Closed {
            panic!("attempted to send stream frame after stream was closed");
        }

        let message = if self.state != StreamState::ReachedEnd {
            self.receiver.try_recv()
        } else {
            self.receiver.recv_timeout(STREAM_RECV_TIMEOUT_MS).await
        };

        if let Some(message) = message {
            match message.header {
                Content(_) => self.handle_content(),
                Closed => self.handle_close(message.bytes),
                Revert => self.handle_revert(message.bytes),
            }
        } else if self.state == StreamState::ReachedEnd {
            #[cfg(feature = "defmt")]
            defmt::error!("timed out waiting for stream close acknowledgement");
            self.state = StreamState::Closed;
            // TODO Notify the callee that the stream transmission likely failed
        } else {
            self.send_data().await;
        }

        self.state == StreamState::Closed
    }

    async fn send_data(&mut self) {
        let offset = self.sequence_id.into_offset(SMTU);
        let data = (self.data_source)(offset).await;

        match data {
            Some(payload) => {
                let seq_id_bytes = self.sequence_id.into_bytes();
                let header = StreamPacket(Content(seq_id_bytes[0]));

                let mut data = [0; TMTU];
                data[0] = header.into();
                data[1] = seq_id_bytes[1];
                data[2] = seq_id_bytes[2];
                data[3..].copy_from_slice(&payload);

                self.transport.send(data).await;
                self.sequence_id.increment();
            }

            None => {
                let header = StreamPacket(Closed);
                let packet = StreamClosePacket {
                    sequence_id: self.sequence_id,
                };

                let mut data = [0; TMTU];
                data[0] = header.into();
                postcard::to_slice(&packet, &mut data[1..])
                    .expect("failed to serialize stream close packet");

                self.transport.send(data).await;
                self.state = StreamState::ReachedEnd;
            }
        }
    }

    fn handle_revert(&mut self, bytes: [u8; PMTU]) {
        match postcard::from_bytes::<StreamRevertPacket>(&bytes) {
            Ok(packet) => {
                self.sequence_id = packet.sequence_id;
            }
            Err(_error) => {
                #[cfg(feature = "defmt")]
                defmt::warn!("failed to deserialize stream revert packet");
            }
        }
    }

    fn handle_close(&mut self, bytes: [u8; PMTU]) {
        if self.state != StreamState::ReachedEnd {
            #[cfg(feature = "defmt")]
            defmt::warn!("dropping unexpected stream end packet");
            return;
        }

        match postcard::from_bytes::<StreamClosePacket>(&bytes) {
            Ok(packet) => {
                if packet.sequence_id == self.sequence_id {
                    self.state = StreamState::Closed;
                } else {
                    #[cfg(feature = "defmt")]
                    defmt::warn!("stream closed with mismatching sequence ID");
                    // TODO Notify the callee that the stream transmission likely failed
                }
            }
            Err(_error) => {
                #[cfg(feature = "defmt")]
                defmt::warn!("failed to deserialize stream close packet");
            }
        }
    }

    fn handle_content(&mut self) {
        #[cfg(feature = "defmt")]
        defmt::warn!("dropping unexpected stream content packet");
    }
}
