use core::{
    future::Future,
    ops::{Add, Deref, Div, Mul, Rem, Sub},
};

use defmt::Format;

pub const BLOCK_SIZE: usize = 512;

pub struct Block {
    content: [u8; BLOCK_SIZE],
}

impl Block {
    pub fn new(content: [u8; BLOCK_SIZE]) -> Self {
        Self { content }
    }
}

impl Deref for Block {
    type Target = [u8; BLOCK_SIZE];

    fn deref(&self) -> &Self::Target {
        &self.content
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Format)]
pub struct BlockID(pub(crate) u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Format)]
pub struct BlockCount(pub(crate) u32);

impl BlockID {
    pub(crate) const ZERO: BlockID = BlockID(0);

    pub fn offset(self) -> usize {
        self.0 as usize * BLOCK_SIZE
    }

    pub fn into_inner(self) -> u32 {
        self.0
    }
}

impl BlockCount {
    /// Creates a new block count and remainder from an offset
    pub fn from_offset(offset: u32) -> (Self, u32) {
        let blocks = offset / BLOCK_SIZE as u32;
        let remainder = offset % BLOCK_SIZE as u32;

        (BlockCount(blocks), remainder)
    }

    pub(crate) fn into_inner(self) -> u32 {
        self.0
    }
}

impl Add<BlockCount> for BlockID {
    type Output = Self;

    fn add(self, rhs: BlockCount) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl Sub<BlockCount> for BlockID {
    type Output = Self;

    fn sub(self, rhs: BlockCount) -> Self::Output {
        Self(self.0 - rhs.0)
    }
}

impl Mul<u8> for BlockCount {
    type Output = Self;

    fn mul(self, rhs: u8) -> Self::Output {
        Self(self.0 * rhs as u32)
    }
}

impl Div for BlockCount {
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        Self(self.0 / rhs.0)
    }
}

impl Rem for BlockCount {
    type Output = Self;

    fn rem(self, rhs: Self) -> Self::Output {
        Self(self.0 % rhs.0)
    }
}

impl Mul for BlockCount {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        Self(self.0 * rhs.0)
    }
}

#[derive(Debug, Format)]
pub enum BlockDeviceError<E> {
    DeviceError(E),
    OutOfBounds,
}

pub struct BlockDevice<E, RFut, RFn, WFut, WFn>
where
    RFut: Future<Output = Result<Block, BlockDeviceError<E>>>,
    RFn: FnMut(BlockID) -> RFut,
    WFut: Future<Output = Result<(), BlockDeviceError<E>>>,
    WFn: FnMut(BlockID, Block) -> WFut,
{
    read_fn: RFn,
    write_fn: WFn,
}

impl<E, RFut, RFn, WFut, WFn> BlockDevice<E, RFut, RFn, WFut, WFn>
where
    RFut: Future<Output = Result<Block, BlockDeviceError<E>>>,
    RFn: Fn(BlockID) -> RFut,
    WFut: Future<Output = Result<(), BlockDeviceError<E>>>,
    WFn: Fn(BlockID, Block) -> WFut,
{
    pub fn new(read_fn: RFn, write_fn: WFn) -> Self {
        Self { read_fn, write_fn }
    }

    pub async fn read(&self, address: BlockID) -> Result<Block, BlockDeviceError<E>> {
        (self.read_fn)(address).await
    }

    pub async fn write(&self, address: BlockID, block: Block) -> Result<(), BlockDeviceError<E>> {
        (self.write_fn)(address, block).await
    }
}
