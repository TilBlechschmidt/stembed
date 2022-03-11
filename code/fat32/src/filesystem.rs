use super::*;
use core::future::Future;
use futures::{pin_mut, Stream, StreamExt, TryStreamExt};

#[derive(Debug)]
pub enum FilesystemError<E> {
    InvalidMasterBootRecord(MasterBootRecordError),
    InvalidVolumeId(VolumeIdError),
    DiskFailure(BlockDeviceError<E>),
    NoFatPartitionFound,
    UnexpectedFatEntry,
    OutOfBounds,
}

pub struct Filesystem<E, RFut, RFn, WFut, WFn>
where
    RFut: Future<Output = Result<Block, BlockDeviceError<E>>>,
    RFn: Fn(BlockID) -> RFut,
    WFut: Future<Output = Result<(), BlockDeviceError<E>>>,
    WFn: Fn(BlockID, Block) -> WFut,
{
    read_fn: RFn,
    write_fn: WFn,

    pub mbr: MasterBootRecord,
    pub partition_index: usize,
    pub vid: VolumeId,
}

impl<E, RFut, RFn, WFut, WFn> Filesystem<E, RFut, RFn, WFut, WFn>
where
    RFut: Future<Output = Result<Block, BlockDeviceError<E>>>,
    RFn: Fn(BlockID) -> RFut,
    WFut: Future<Output = Result<(), BlockDeviceError<E>>>,
    WFn: Fn(BlockID, Block) -> WFut,
{
    pub async fn new(read_fn: RFn, write_fn: WFn) -> Result<Self, FilesystemError<E>> {
        let mbr_block = (read_fn)(BlockID::ZERO)
            .await
            .map_err(FilesystemError::DiskFailure)?;
        let mbr = MasterBootRecord::try_from(mbr_block)
            .map_err(FilesystemError::InvalidMasterBootRecord)?;

        let fat_partition = mbr
            .partitions
            .iter()
            .enumerate()
            .find(|(_, partition)| partition.partition_type == PartitionType::FAT32);

        match fat_partition {
            Some((partition_index, partition)) => {
                let vid_block = (read_fn)(partition.block_address)
                    .await
                    .map_err(FilesystemError::DiskFailure)?;

                let vid = VolumeId::try_from((partition.block_address, vid_block))
                    .map_err(FilesystemError::InvalidVolumeId)?;

                Ok(Self {
                    read_fn,
                    write_fn,
                    mbr,
                    partition_index,
                    vid,
                })
            }
            None => Err(FilesystemError::NoFatPartitionFound),
        }
    }

    pub(crate) async fn read(&self, address: BlockID) -> Result<Block, FilesystemError<E>> {
        (self.read_fn)(address)
            .await
            .map_err(FilesystemError::DiskFailure)
    }

    async fn next_cluster(
        &self,
        current_cluster: ClusterID,
    ) -> Result<Option<ClusterID>, FilesystemError<E>> {
        let fat_entry_offset = current_cluster.as_fat_offset();
        let fat_entry_block_offset = BlockCount((fat_entry_offset / BLOCK_SIZE) as u32);
        let fat_entry_intra_block_offset = fat_entry_offset % BLOCK_SIZE;

        let fat_address = self.vid.fat_address();
        let entry_address = fat_address + fat_entry_block_offset;
        let entry_block = self.read(entry_address).await?;

        let next_entry = u32::from_le_bytes([
            entry_block[fat_entry_intra_block_offset],
            entry_block[fat_entry_intra_block_offset + 1],
            entry_block[fat_entry_intra_block_offset + 2],
            entry_block[fat_entry_intra_block_offset + 3],
        ]);

        if next_entry >= 0xFFF_FFFF {
            Ok(None)
        } else if next_entry == 0x0 {
            Err(FilesystemError::UnexpectedFatEntry)
        } else {
            Ok(Some(ClusterID(next_entry)))
        }
    }

    pub(crate) fn cluster_chain(
        &self,
        start_cluster: ClusterID,
    ) -> impl Stream<Item = Result<ClusterID, FilesystemError<E>>> + '_ {
        futures::stream::once(async move { Ok(start_cluster) }).chain(futures::stream::unfold(
            start_cluster,
            move |address| async move {
                let next = self.next_cluster(address).await;
                match next {
                    Ok(Some(id)) => Some((Ok(id), id)),
                    Ok(None) => None,
                    Err(err) => Some((Err(err), address)),
                }
            },
        ))
    }

    fn block_chain(
        &self,
        start_cluster: ClusterID,
    ) -> impl Stream<Item = Result<Block, FilesystemError<E>>> + '_ {
        self.cluster_chain(start_cluster)
            .map_ok(|address| self.vid.cluster_address(address))
            .then(move |result| async move {
                match result {
                    Ok(address) => self.read(address).await,
                    Err(error) => Err(error),
                }
            })
    }

    pub fn enumerate_directory(
        &self,
        directory: Directory,
    ) -> impl Stream<Item = Result<DirectoryEntry, FilesystemError<E>>> + '_ {
        let block_stream = self.block_chain(directory.cluster);
        block_stream_to_entry_stream(block_stream)
    }

    pub fn root_directory(&self) -> Directory {
        Directory {
            // TODO Figure out the name from the volume ID file in the root directory
            name: Name::new(*b"ROOT_DIR   "),
            cluster: self.vid.root_directory_cluster(),
        }
    }

    /// Currently only searches in the root directory
    pub async fn find_file(
        &self,
        name: &str,
        extension: &str,
    ) -> Result<Option<File>, FilesystemError<E>> {
        let entries = self.enumerate_directory(self.root_directory());
        pin_mut!(entries);

        while let Some(entry) = entries.next().await {
            match entry? {
                DirectoryEntry::File(file) => {
                    if file.name().name() == Ok(name) && file.name().extension() == Ok(extension) {
                        return Ok(Some(file));
                    }
                }
                _ => continue,
            }
        }

        Ok(None)
    }

    pub fn volume_id(&self) -> &VolumeId {
        &self.vid
    }
}

fn block_stream_to_entry_stream<E>(
    stream: impl Stream<Item = Result<Block, FilesystemError<E>>>,
) -> impl Stream<Item = Result<DirectoryEntry, FilesystemError<E>>> {
    stream
        .map_ok(block_to_entry_stream)
        .try_flatten()
        .take_while(not_end_of_directory)
        .try_filter_map(take_regular_entries)
}

fn block_to_entry_stream<E>(
    block: Block,
) -> impl Stream<Item = Result<DirectoryIndexEntry, FilesystemError<E>>> {
    futures::stream::iter(
        (0..DIRECTORY_ENTRIES_PER_BLOCK)
            .map(move |entry_index| {
                let start = entry_index * DIRECTORY_ENTRY_SIZE;
                let end = (entry_index + 1) * DIRECTORY_ENTRY_SIZE;
                let data = &block[start..end];
                DirectoryIndexEntry::from(data)
            })
            .map(Ok),
    )
}

async fn take_regular_entries<E>(
    entry: DirectoryIndexEntry,
) -> Result<Option<DirectoryEntry>, FilesystemError<E>> {
    match entry {
        DirectoryIndexEntry::Entry(regular_entry) => Ok(Some(regular_entry)),
        DirectoryIndexEntry::Unused => Ok(None),
        DirectoryIndexEntry::EndOfDirectory => Ok(None),
    }
}

fn not_end_of_directory<E>(
    item: &Result<DirectoryIndexEntry, FilesystemError<E>>,
) -> impl core::future::Future<Output = bool> {
    let is_eod = match item {
        Ok(DirectoryIndexEntry::EndOfDirectory) => false,
        _ => true,
    };

    async move { is_eod }
}
