use defmt::Format;

use crate::{Block, BlockCount, BlockID, BLOCK_SIZE};
use core::ops::Sub;

#[derive(Format, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClusterID(pub(crate) u32);

#[derive(Format, Debug)]
pub enum VolumeIdError {
    InvalidBlockSize,
    InvalidSignature,
}

#[derive(Debug, Format)]
pub struct VolumeId {
    partition_address: BlockID,

    sectors_per_cluster: BlockCount,
    reserved_sectors: BlockCount,

    number_of_fats: u8,
    sectors_per_fat: BlockCount,

    root_dir_cluster_id: ClusterID,
}

impl VolumeId {
    pub fn fat_address(&self) -> BlockID {
        self.partition_address + self.reserved_sectors
    }

    pub fn first_cluster_address(&self) -> BlockID {
        self.partition_address
            + self.reserved_sectors
            + (self.sectors_per_fat * self.number_of_fats)
    }

    pub fn cluster_address(&self, cluster_id: ClusterID) -> BlockID {
        self.first_cluster_address()
            + ((cluster_id - 2).as_block_count_unchecked() * self.sectors_per_cluster)
    }

    pub fn cluster_address_from_offset(&self, start: ClusterID, offset: BlockCount) -> BlockID {
        self.cluster_address(start) + offset
    }

    pub fn cluster_offset(&self, offset: BlockCount) -> u32 {
        (offset / self.sectors_per_cluster).into_inner()
    }

    pub fn intra_cluster_offset(&self, offset: BlockCount) -> BlockCount {
        offset % self.sectors_per_cluster
    }

    pub fn root_directory_address(&self) -> BlockID {
        self.cluster_address(self.root_dir_cluster_id)
    }

    pub fn root_directory_cluster(&self) -> ClusterID {
        self.root_dir_cluster_id
    }
}

impl TryFrom<(BlockID, Block)> for VolumeId {
    type Error = VolumeIdError;

    fn try_from((partition_address, block): (BlockID, Block)) -> Result<Self, Self::Error> {
        defmt::trace!("{:?}", *block);
        let sector_size = u16::from_le_bytes([block[0x0B], block[0x0B + 1]]);

        if sector_size != BLOCK_SIZE as u16 {
            defmt::warn!(
                "Expected block size of {} did not match actual {}",
                BLOCK_SIZE,
                sector_size
            );
            return Err(VolumeIdError::InvalidBlockSize);
        }

        let sectors_per_cluster = BlockCount(block[0x0D] as u32);
        let reserved_sectors =
            BlockCount(u16::from_le_bytes([block[0x0E], block[0x0E + 1]]) as u32);

        let number_of_fats = block[0x10];
        let sectors_per_fat = BlockCount(u32::from_le_bytes([
            block[0x24],
            block[0x24 + 1],
            block[0x24 + 2],
            block[0x24 + 3],
        ]));

        let root_dir_cluster_id =
            ClusterID(u16::from_le_bytes([block[0x2C], block[0x2C + 1]]) as u32);

        let signature = u16::from_le_bytes([block[0x1FE], block[0x1FE + 1]]);

        if signature != 0xAA55 {
            Err(VolumeIdError::InvalidSignature)
        } else {
            Ok(Self {
                partition_address,
                sectors_per_cluster,
                reserved_sectors,
                number_of_fats,
                sectors_per_fat,
                root_dir_cluster_id,
            })
        }
    }
}

impl ClusterID {
    /// Performs a direct type-conversion into BlockCount
    /// without checking anything. This is usually not what you want
    /// because cluster addresses do not directly relate to block counts.
    /// Instead, they have to be multiplied by the number of blocks per cluster!
    fn as_block_count_unchecked(self) -> BlockCount {
        BlockCount(self.0)
    }

    /// Converts the ClusterID into an offset in the FAT where
    /// the pointer to the next ClusterID can be found
    pub(crate) fn as_fat_offset(self) -> usize {
        (self.0 * u32::BITS / 8) as usize
    }
}

impl Sub<u32> for ClusterID {
    type Output = Self;

    fn sub(self, rhs: u32) -> Self::Output {
        Self(self.0 - rhs)
    }
}
