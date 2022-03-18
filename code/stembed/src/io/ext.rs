use super::{Read, Result, Seek, SeekFrom, Write};
use core::future::Future;

pub trait ReadExt {
    type ReadU16Future<'a>: Future<Output = Result<u16>> + 'a
    where
        Self: 'a;

    type ReadU32Future<'a>: Future<Output = Result<u32>> + 'a
    where
        Self: 'a;

    fn read_u16(&mut self) -> Self::ReadU16Future<'_>;
    fn read_u32(&mut self) -> Self::ReadU32Future<'_>;
}

pub trait WriteExt {
    type WriteU16Future<'a>: Future<Output = Result<()>> + 'a
    where
        Self: 'a;

    type WriteU32Future<'a>: Future<Output = Result<()>> + 'a
    where
        Self: 'a;

    fn write_u16(&mut self, data: u16) -> Self::WriteU16Future<'_>;
    fn write_u32(&mut self, data: u32) -> Self::WriteU32Future<'_>;
}

pub trait SeekExt {
    type StreamPositionFuture<'a>: Future<Output = Result<u64>> + 'a
    where
        Self: 'a;

    type StreamLengthFuture<'a>: Future<Output = Result<u64>> + 'a
    where
        Self: 'a;

    fn stream_position(&mut self) -> Self::StreamPositionFuture<'_>;
    fn stream_len(&mut self) -> Self::StreamLengthFuture<'_>;
}

impl<W> WriteExt for W
where
    W: Write,
{
    type WriteU16Future<'a> = impl Future<Output = Result<()>> + 'a where Self: 'a;
    type WriteU32Future<'a> = impl Future<Output = Result<()>> + 'a where Self: 'a;

    fn write_u16(&mut self, data: u16) -> Self::WriteU16Future<'_> {
        async move {
            for byte in data.to_be_bytes().into_iter() {
                self.write(byte).await?;
            }
            Ok(())
        }
    }

    fn write_u32(&mut self, data: u32) -> Self::WriteU32Future<'_> {
        async move {
            for byte in data.to_be_bytes().into_iter() {
                self.write(byte).await?;
            }
            Ok(())
        }
    }
}

impl<R> ReadExt for R
where
    R: Read,
{
    type ReadU16Future<'a> = impl Future<Output = Result<u16>> + 'a where Self: 'a;
    type ReadU32Future<'a> = impl Future<Output = Result<u32>> + 'a where Self: 'a;

    fn read_u16(&mut self) -> Self::ReadU16Future<'_> {
        async move { Ok(u16::from_be_bytes([self.read().await?, self.read().await?])) }
    }

    fn read_u32(&mut self) -> Self::ReadU32Future<'_> {
        async move {
            Ok(u32::from_be_bytes([
                self.read().await?,
                self.read().await?,
                self.read().await?,
                self.read().await?,
            ]))
        }
    }
}

impl<S> SeekExt for S
where
    S: Seek,
{
    type StreamPositionFuture<'a> = impl Future<Output = Result<u64>> + 'a where Self: 'a;
    type StreamLengthFuture<'a> = impl Future<Output = Result<u64>> + 'a where Self: 'a;

    fn stream_position(&mut self) -> Self::StreamPositionFuture<'_> {
        self.seek(SeekFrom::Current(0))
    }

    fn stream_len(&mut self) -> Self::StreamLengthFuture<'_> {
        async move {
            let old_pos = self.stream_position().await?;
            let len = self.seek(SeekFrom::End(0)).await?;

            // Avoid seeking a third time when we were already at the end of the
            // stream. The branch is usually way cheaper than a seek operation.
            if old_pos != len {
                self.seek(SeekFrom::Start(old_pos)).await?;
            }

            Ok(len)
        }
    }
}
