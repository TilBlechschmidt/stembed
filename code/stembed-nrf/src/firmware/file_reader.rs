use fat32::{Block, BlockDeviceError, BlockID, FileReader};
use futures::Future;
use stembed::io::{Read, Seek, SeekFrom};

pub struct Reader<'f, E, RFut, RFn, WFut, WFn>
where
    RFut: Future<Output = Result<Block, BlockDeviceError<E>>>,
    RFn: Fn(BlockID) -> RFut,
    WFut: Future<Output = Result<(), BlockDeviceError<E>>>,
    WFn: Fn(BlockID, Block) -> WFut,
{
    file: FileReader<'f, E, RFut, RFn, WFut, WFn>,
    offset: u32,
}

impl<'f, E, RFut, RFn, WFut, WFn> Reader<'f, E, RFut, RFn, WFut, WFn>
where
    RFut: Future<Output = Result<Block, BlockDeviceError<E>>>,
    RFn: Fn(BlockID) -> RFut,
    WFut: Future<Output = Result<(), BlockDeviceError<E>>>,
    WFn: Fn(BlockID, Block) -> WFut,
{
    pub fn new(file: FileReader<'f, E, RFut, RFn, WFut, WFn>) -> Self {
        Self { file, offset: 0 }
    }
}

impl<'f, E, RFut, RFn, WFut, WFn> Read for Reader<'f, E, RFut, RFn, WFut, WFn>
where
    RFut: Future<Output = Result<Block, BlockDeviceError<E>>>,
    RFn: Fn(BlockID) -> RFut,
    WFut: Future<Output = Result<(), BlockDeviceError<E>>>,
    WFn: Fn(BlockID, Block) -> WFut,
{
    type ReadFuture<'a> = impl Future<Output = Result<u8, stembed::io::Error>> + 'a
    where
        Self: 'a;

    fn read(&mut self) -> Self::ReadFuture<'_> {
        async move {
            let data = self.file.read(self.offset).await;
            self.offset += 1;
            data.map_err(|_e| stembed::io::Error::Unknown)
        }
    }
}

impl<'f, E, RFut, RFn, WFut, WFn> Seek for Reader<'f, E, RFut, RFn, WFut, WFn>
where
    RFut: Future<Output = Result<Block, BlockDeviceError<E>>>,
    RFn: Fn(BlockID) -> RFut,
    WFut: Future<Output = Result<(), BlockDeviceError<E>>>,
    WFn: Fn(BlockID, Block) -> WFut,
{
    type SeekFuture<'a> = impl Future<Output = Result<u64, stembed::io::Error>> + 'a
    where
        Self: 'a;

    fn seek(&mut self, pos: SeekFrom) -> Self::SeekFuture<'_> {
        async move {
            match pos {
                SeekFrom::Start(offset) => self.offset = offset as u32,
                SeekFrom::End(offset) => unimplemented!(),
                SeekFrom::Current(offset) => self.offset = (self.offset as i64 + offset) as u32,
            }

            // TODO Check for out-of-bounds

            Ok(self.offset as u64)
        }
    }
}
