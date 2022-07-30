use cofit::{Message, MessageIdentifier};

/// Erases a range of sectors in the flash back to `1`
pub struct EraseFlash<const MTU: usize> {
    pub start_sector: u16,
    pub end_sector: u16,
}

/// Confirms that the given sectors have been erased
pub struct FlashErased<const MTU: usize> {
    pub start_sector: u16,
    pub end_sector: u16,
}

impl<const MTU: usize> Message<MTU> for EraseFlash<MTU> {
    const IDENTIFIER: MessageIdentifier<'static> = "flash.erase";

    fn to_packet(self) -> [u8; MTU] {
        to_packet(self.start_sector, self.end_sector)
    }

    fn from_packet(packet: [u8; MTU]) -> Result<Self, ()> {
        let (start_sector, end_sector) = from_packet(packet);

        Ok(Self {
            start_sector,
            end_sector,
        })
    }
}

impl<const MTU: usize> Message<MTU> for FlashErased<MTU> {
    const IDENTIFIER: MessageIdentifier<'static> = "flash.erase.ack";

    fn to_packet(self) -> [u8; MTU] {
        to_packet(self.start_sector, self.end_sector)
    }

    fn from_packet(packet: [u8; MTU]) -> Result<Self, ()> {
        let (start_sector, end_sector) = from_packet(packet);

        Ok(Self {
            start_sector,
            end_sector,
        })
    }
}

impl<const MTU: usize> From<EraseFlash<MTU>> for FlashErased<MTU> {
    fn from(erase: EraseFlash<MTU>) -> Self {
        Self {
            start_sector: erase.start_sector,
            end_sector: erase.end_sector,
        }
    }
}

#[inline]
fn to_packet<const MTU: usize>(start_sector: u16, end_sector: u16) -> [u8; MTU] {
    let mut packet = [0; MTU];
    let start_bytes = start_sector.to_be_bytes();
    let end_bytes = end_sector.to_be_bytes();
    packet[0..2].copy_from_slice(&start_bytes);
    packet[2..4].copy_from_slice(&end_bytes);
    packet
}

#[inline]
fn from_packet<const MTU: usize>(packet: [u8; MTU]) -> (u16, u16) {
    let start_sector = u16::from_be_bytes([packet[0], packet[1]]);
    let end_sector = u16::from_be_bytes([packet[2], packet[3]]);
    (start_sector, end_sector)
}
