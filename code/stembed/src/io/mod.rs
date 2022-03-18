use core::future::Future;

mod ext;
pub use ext::*;

pub mod util;

#[derive(Debug)]
pub enum Error {
    EOF,
    Unknown,
}

#[derive(Debug)]
pub enum SeekFrom {
    Start(u64),
    End(i64),
    Current(i64),
}

type Result<V> = core::result::Result<V, Error>;

pub trait Write {
    type WriteFuture<'a>: Future<Output = Result<()>> + 'a
    where
        Self: 'a;

    fn write(&mut self, data: u8) -> Self::WriteFuture<'_>;
}

pub trait Read {
    type ReadFuture<'a>: Future<Output = Result<u8>> + 'a
    where
        Self: 'a;

    fn read(&mut self) -> Self::ReadFuture<'_>;
}

pub trait Seek {
    type SeekFuture<'a>: Future<Output = Result<u64>> + 'a
    where
        Self: 'a;

    fn seek(&mut self, pos: SeekFrom) -> Self::SeekFuture<'_>;
}
