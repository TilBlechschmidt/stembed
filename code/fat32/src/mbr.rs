use defmt::Format;

use crate::{Block, BlockCount, BlockID};

#[derive(Format, Debug, Clone, Copy, PartialEq, Eq)]
pub enum PartitionType {
    FAT32,
    Unknown,
}

#[derive(Debug, Format)]
pub struct Partition {
    pub partition_type: PartitionType,
    pub block_address: BlockID,
    pub sector_count: BlockCount,
}

impl From<&[u8]> for Partition {
    fn from(mbr_entry: &[u8]) -> Self {
        assert_eq!(mbr_entry.len(), 16);

        let partition_type = match mbr_entry[0x04] {
            0x0B | 0x0C => PartitionType::FAT32,
            _ => PartitionType::Unknown,
        };

        let block_address = BlockID(u32::from_le_bytes([
            mbr_entry[0x08],
            mbr_entry[0x09],
            mbr_entry[0x0A],
            mbr_entry[0x0B],
        ]));

        let sector_count = BlockCount(u32::from_le_bytes([
            mbr_entry[0x0C],
            mbr_entry[0x0D],
            mbr_entry[0x0E],
            mbr_entry[0x0F],
        ]));

        Self {
            partition_type,
            block_address,
            sector_count,
        }
    }
}

#[derive(Debug, Format)]
pub enum MasterBootRecordError {
    InvalidSignature,
}

#[derive(Debug, Format)]
pub struct MasterBootRecord {
    pub partitions: [Partition; 4],
}

impl TryFrom<Block> for MasterBootRecord {
    type Error = MasterBootRecordError;

    fn try_from(block: Block) -> Result<Self, Self::Error> {
        let partitions = [
            Partition::from(&block[0x01BE..0x01BE + 16]),
            Partition::from(&block[0x01CE..0x01CE + 16]),
            Partition::from(&block[0x01DE..0x01DE + 16]),
            Partition::from(&block[0x01EE..0x01EE + 16]),
        ];

        let signature_valid = block[0x01FE] == 0x55 && block[0x01FF] == 0xAA;

        if !signature_valid {
            Err(MasterBootRecordError::InvalidSignature)
        } else {
            Ok(Self { partitions })
        }
    }
}
