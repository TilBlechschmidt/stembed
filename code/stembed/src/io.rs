use alloc::vec::Vec;
use core::hash::Hasher;

#[derive(Debug)]
pub enum Error {
    EOF,
    Unknown,
}

type Result<V> = core::result::Result<V, Error>;

pub trait Write {
    fn write_u8(&mut self, data: u8) -> Result<()>;

    fn write_u16(&mut self, data: u16) -> Result<()> {
        for byte in data.to_be_bytes().into_iter() {
            self.write_u8(byte)?;
        }
        Ok(())
    }

    fn write_u32(&mut self, data: u32) -> Result<()> {
        for byte in data.to_be_bytes().into_iter() {
            self.write_u8(byte)?;
        }
        Ok(())
    }
}

pub trait Read {
    fn read_u8(&mut self) -> Result<u8>;

    fn read_u16(&mut self) -> Result<u16> {
        Ok(u16::from_be_bytes([self.read_u8()?, self.read_u8()?]))
    }

    fn read_u32(&mut self) -> Result<u32> {
        Ok(u32::from_be_bytes([
            self.read_u8()?,
            self.read_u8()?,
            self.read_u8()?,
            self.read_u8()?,
        ]))
    }
}

pub enum SeekFrom {
    Start(u64),
    End(i64),
    Current(i64),
}

pub trait Seek {
    fn seek(&mut self, pos: SeekFrom) -> Result<u64>;

    fn stream_position(&mut self) -> Result<u64> {
        self.seek(SeekFrom::Current(0))
    }

    fn stream_len(&mut self) -> Result<u64> {
        let old_pos = self.stream_position()?;
        let len = self.seek(SeekFrom::End(0))?;

        // Avoid seeking a third time when we were already at the end of the
        // stream. The branch is usually way cheaper than a seek operation.
        if old_pos != len {
            self.seek(SeekFrom::Start(old_pos))?;
        }

        Ok(len)
    }
}

impl<H> Write for H
where
    H: Hasher,
{
    fn write_u8(&mut self, data: u8) -> Result<()> {
        self.write(&[data]);
        Ok(())
    }
}

impl<I> Read for I
where
    I: Iterator<Item = u8>,
{
    fn read_u8(&mut self) -> Result<u8> {
        self.next().ok_or(Error::EOF)
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
    fn write_u8(&mut self, data: u8) -> Result<()> {
        self.data.push(data);
        Ok(())
    }
}

impl Read for HeapFile {
    fn read_u8(&mut self) -> Result<u8> {
        let output = self
            .data
            .get(self.position as usize)
            .cloned()
            .ok_or(Error::EOF);

        self.position += 1;

        output
    }
}

impl Seek for HeapFile {
    fn seek(&mut self, pos: SeekFrom) -> Result<u64> {
        match pos {
            SeekFrom::Start(offset) => self.position = offset,
            SeekFrom::End(offset) => self.position = (self.data.len() as i64 - offset) as u64,
            SeekFrom::Current(offset) => self.position = (self.position as i64 + offset) as u64,
        };

        Ok(self.position)
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
    fn write_u8(&mut self, data: u8) -> Result<()> {
        self.bytes += 1;
        self.writer.write_u8(data)
    }
}
