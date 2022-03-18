use super::{Error, Read, Result, Seek, SeekFrom, Write};
use alloc::vec::Vec;
use core::{future::Future, hash::Hasher};

impl<H> Write for H
where
    H: Hasher,
{
    type WriteFuture<'a> = impl Future<Output = Result<()>> + 'a where Self: 'a;

    fn write(&mut self, data: u8) -> Self::WriteFuture<'_> {
        async move {
            self.write(&[data]);
            Ok(())
        }
    }
}

impl<I> Read for I
where
    I: Iterator<Item = u8>,
{
    type ReadFuture<'a> = impl Future<Output = Result<u8>> + 'a where Self: 'a;

    fn read(&mut self) -> Self::ReadFuture<'_> {
        async move { self.next().ok_or(Error::EOF) }
    }
}

pub struct HeapFile {
    data: Vec<u8>,
    position: u64,
}

impl HeapFile {
    pub fn new() -> Self {
        Self {
            data: Vec::new(),
            position: 0,
        }
    }

    pub fn from_raw(data: Vec<u8>) -> Self {
        Self { data, position: 0 }
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.data
    }

    pub fn into_inner(self) -> Vec<u8> {
        self.data
    }
}

impl Write for HeapFile {
    type WriteFuture<'a> = impl Future<Output = Result<()>> + 'a where Self: 'a;

    fn write(&mut self, data: u8) -> Self::WriteFuture<'_> {
        async move {
            self.data.push(data);
            Ok(())
        }
    }
}

impl Read for HeapFile {
    type ReadFuture<'a> = impl Future<Output = Result<u8>> + 'a where Self: 'a;

    fn read(&mut self) -> Self::ReadFuture<'_> {
        async move {
            let output = self
                .data
                .get(self.position as usize)
                .cloned()
                .ok_or(Error::EOF);

            self.position += 1;

            output
        }
    }
}

impl Seek for HeapFile {
    type SeekFuture<'a> = impl Future<Output = Result<u64>> + 'a where Self: 'a;

    fn seek(&mut self, pos: SeekFrom) -> Self::SeekFuture<'_> {
        async move {
            match pos {
                SeekFrom::Start(offset) => self.position = offset,
                SeekFrom::End(offset) => self.position = (self.data.len() as i64 - offset) as u64,
                SeekFrom::Current(offset) => self.position = (self.position as i64 + offset) as u64,
            };

            Ok(self.position)
        }
    }
}

pub(crate) struct CountingWriter<'w, W: Write> {
    writer: &'w mut W,
    bytes: u64,
}

impl<'w, W: Write> CountingWriter<'w, W> {
    pub(crate) fn new(writer: &'w mut W) -> Self {
        Self { writer, bytes: 0 }
    }

    pub(crate) fn position(&self) -> u64 {
        self.bytes
    }
}

impl<'w, W: Write> Write for CountingWriter<'w, W> {
    type WriteFuture<'a> = impl Future<Output = Result<()>> + 'a where Self: 'a;

    fn write(&mut self, data: u8) -> Self::WriteFuture<'_> {
        self.bytes += 1;
        self.writer.write(data)
    }
}
