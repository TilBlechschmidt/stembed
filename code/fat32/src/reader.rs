use core::future::Future;
use futures::{pin_mut, StreamExt};

use crate::{
    Block, BlockCount, BlockDeviceError, BlockID, ClusterID, File, Filesystem, FilesystemError,
    BLOCK_SIZE,
};

pub struct FileReader<'f, E, RFut, RFn, WFut, WFn>
where
    RFut: Future<Output = Result<Block, BlockDeviceError<E>>>,
    RFn: Fn(BlockID) -> RFut,
    WFut: Future<Output = Result<(), BlockDeviceError<E>>>,
    WFn: Fn(BlockID, Block) -> WFut,
{
    file: File,
    filesystem: &'f Filesystem<E, RFut, RFn, WFut, WFn>,
    block_cache: (BlockID, Block),
    cluster_cache: Option<[ClusterID; 103]>,
}

impl<'f, E, RFut, RFn, WFut, WFn> FileReader<'f, E, RFut, RFn, WFut, WFn>
where
    RFut: Future<Output = Result<Block, BlockDeviceError<E>>>,
    RFn: Fn(BlockID) -> RFut,
    WFut: Future<Output = Result<(), BlockDeviceError<E>>>,
    WFn: Fn(BlockID, Block) -> WFut,
{
    // TODO Have the file retain a reference to the filesystem so it can have an `open` method which returns a FileReader
    pub fn new(file: File, filesystem: &'f Filesystem<E, RFut, RFn, WFut, WFn>) -> Self {
        Self {
            block_cache: (BlockID::ZERO, Block::new([0; BLOCK_SIZE])),
            cluster_cache: None,
            file,
            filesystem,
        }
    }

    pub async fn cache_fat(&mut self) -> Result<(), FilesystemError<E>> {
        let cluster_chain = self.filesystem.cluster_chain(self.file.cluster_address());

        pin_mut!(cluster_chain);

        let mut cache = [self.file.cluster_address(); 103];
        let mut cluster_count = 0;
        while let Some(cluster_id) = cluster_chain.next().await {
            cache[cluster_count] = cluster_id?;
            cluster_count += 1;
        }

        self.cluster_cache = Some(cache);

        Ok(())
    }

    pub async fn read(&mut self, offset: u32) -> Result<u8, FilesystemError<E>> {
        if offset > self.file.size() as u32 {
            return Err(FilesystemError::OutOfBounds);
        }

        let vid = self.filesystem.volume_id();

        // 1. Calculate cluster ID and intra-cluster offset
        let (block_offset, intra_block_offset) = BlockCount::from_offset(offset);
        let cluster_offset = vid.cluster_offset(block_offset);
        let intra_cluster_offset = vid.intra_cluster_offset(block_offset);

        // 2. Find ClusterID of the cluster containing our offset
        let cluster_id = if let Some(cache) = self.cluster_cache.as_ref() {
            cache[cluster_offset as usize]
        } else {
            let cluster_chain = self
                .filesystem
                .cluster_chain(self.file.cluster_address())
                .skip(cluster_offset as usize);

            pin_mut!(cluster_chain);

            cluster_chain
                .next()
                .await
                .ok_or(FilesystemError::OutOfBounds)??
        };

        // 3. Calculate BlockID of our data based on the ClusterID and the offset within the cluster
        let cluster_block_id = self.filesystem.volume_id().cluster_address(cluster_id);
        let data_block_id = cluster_block_id + intra_cluster_offset;

        // 4. Load cache or fetch and cache block
        let block = if self.block_cache.0 == data_block_id {
            &self.block_cache.1
        } else {
            self.block_cache = (data_block_id, self.filesystem.read(data_block_id).await?);
            &self.block_cache.1
        };

        // 5. Read from block
        Ok(block[intra_block_offset as usize])
    }
}
