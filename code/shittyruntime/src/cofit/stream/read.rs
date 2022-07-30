use super::{
    super::{PacketHeader, StreamReceiverLock, STREAM_RECV_TIMEOUT_MS},
    MpscReceiver, StreamClosePacket, StreamPacketHeader, StreamRevertPacket, StreamSequenceID,
    Transport,
};
use StreamPacketHeader::*;

pub struct StreamReadHandle<
    't,
    const TMTU: usize,
    const PMTU: usize,
    const SMTU: usize,
    T: Transport<TMTU>,
> {
    receiver: StreamReceiverLock<'t, PMTU>,
    transport: &'t T,

    sequence_id: StreamSequenceID,
    reached_end: bool,
}

impl<'t, const TMTU: usize, const PMTU: usize, const SMTU: usize, T: Transport<TMTU>>
    StreamReadHandle<'t, TMTU, PMTU, SMTU, T>
{
    pub fn new(receiver: StreamReceiverLock<'t, PMTU>, transport: &'t T) -> Self {
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
            sequence_id: StreamSequenceID(0),
            reached_end: false,
        }
    }

    pub async fn recv(&mut self) -> Option<[u8; SMTU]> {
        loop {
            match self.receiver.recv_timeout(STREAM_RECV_TIMEOUT_MS).await {
                Some(message) => match message.header {
                    Content(seq_id_byte) => {
                        if let Some(data) = self.handle_content(seq_id_byte, message.bytes).await {
                            return Some(data);
                        }
                    }
                    Closed => {
                        self.handle_close(message.bytes).await;
                        if self.reached_end {
                            return None;
                        }
                    }
                    Revert => self.handle_revert(),
                },
                None => {
                    #[cfg(feature = "defmt")]
                    defmt::error!("timed out while waiting for stream packet");
                    return None;
                }
            }
        }
    }

    async fn handle_content(&mut self, seq_id_byte: u8, bytes: [u8; PMTU]) -> Option<[u8; SMTU]> {
        let seq_id = StreamSequenceID::from_bytes([seq_id_byte, bytes[0], bytes[1]]);

        if seq_id > self.sequence_id {
            self.request_revert().await;
            return None;
        } else if seq_id < self.sequence_id {
            #[cfg(feature = "defmt")]
            defmt::warn!("encountered discontinuity in sequence IDs");
            return None;
        }

        self.sequence_id.increment();

        let mut data = [0; SMTU];
        data.copy_from_slice(&bytes[2..]);
        Some(data)
    }

    async fn handle_close(&mut self, bytes: [u8; PMTU]) {
        match postcard::from_bytes::<StreamClosePacket>(&bytes) {
            Ok(packet) => {
                if packet.sequence_id == self.sequence_id {
                    self.acknowledge_close().await;
                    self.reached_end = true;
                } else if packet.sequence_id < self.sequence_id {
                    #[cfg(feature = "defmt")]
                    defmt::warn!("encountered discontinuity while closing stream");
                    // TODO Notify the callee as data corruption might have occurred, maybe even notify the host somehow
                } else {
                    self.request_revert().await;
                }
            }
            Err(_error) => {
                #[cfg(feature = "defmt")]
                defmt::warn!("failed to deserialize stream close packet");
            }
        }
    }

    fn handle_revert(&mut self) {
        #[cfg(feature = "defmt")]
        defmt::warn!("dropping unexpected stream revert packet");
    }

    async fn request_revert(&mut self) {
        let header = PacketHeader::StreamPacket(Revert);
        let packet = StreamRevertPacket {
            sequence_id: self.sequence_id,
        };

        let mut data = [0; TMTU];
        data[0] = header.into();
        postcard::to_slice(&packet, &mut data[1..])
            .expect("failed to serialize stream revert packet");

        self.transport.send(data).await;
    }

    async fn acknowledge_close(&mut self) {
        let header = PacketHeader::StreamPacket(Closed);
        let packet = StreamClosePacket {
            sequence_id: self.sequence_id,
        };

        let mut data = [0; TMTU];
        data[0] = header.into();
        postcard::to_slice(&packet, &mut data[1..])
            .expect("failed to serialize stream revert packet");

        self.transport.send(data).await;
    }
}
