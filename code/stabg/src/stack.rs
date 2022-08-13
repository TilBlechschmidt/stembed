use crate::ShortID;
use core::ops::Range;

#[derive(Debug)]
pub enum Error {
    StackOverflow,
    ValueTooLarge,
}

pub trait Stack {
    fn clear(&mut self);
    fn push(&mut self, code: ShortID, data: &[u8]) -> Result<(), Error>;
    fn pop(&mut self) -> Option<(ShortID, &[u8])>;
    fn get(&self, code: ShortID) -> Option<&[u8]>;

    // TODO Find a way to return one iterator type which works for all implementations (by holding a ref)
    fn iter_reset(&mut self);
    fn iter_next(&mut self) -> Option<(ShortID, &[u8])>;
}

/// Slice based stack implementation that stores entries with a 3-byte heder
/// containing length (u16) and ShortID (u8)
pub struct FixedSizeStack<const CAPACITY: usize> {
    data: [u8; CAPACITY],
    usage: usize,
    iter_pointer: usize,
}

impl<const CAPACITY: usize> FixedSizeStack<CAPACITY> {
    const HEADER_SIZE: usize = 3;

    /// Number of extra bytes required per value
    pub const OVERHEAD: usize = Self::HEADER_SIZE;

    pub fn new() -> Self {
        Self {
            data: [0; CAPACITY],
            usage: 0,
            iter_pointer: 0,
        }
    }

    pub fn free(&self) -> usize {
        self.data.len() - self.usage
    }

    /// Fetches the header for the entry located immediately before the given offset
    fn fetch_header_from(&self, offset: usize) -> Option<(ShortID, Range<usize>)> {
        if offset < Self::HEADER_SIZE {
            None
        } else {
            let id: ShortID = self.data[offset - 1].into();
            let length = u16::from_be_bytes([self.data[offset - 3], self.data[offset - 2]]);
            let range = offset - Self::HEADER_SIZE - length as usize..offset - Self::HEADER_SIZE;
            Some((id, range))
        }
    }
}

impl<const CAPACITY: usize> Default for FixedSizeStack<CAPACITY> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const CAPACITY: usize> Stack for FixedSizeStack<CAPACITY> {
    fn clear(&mut self) {
        self.usage = 0;
        self.iter_pointer = 0;
    }

    fn push(&mut self, code: ShortID, data: &[u8]) -> Result<(), Error> {
        if self.free() < data.len() + Self::HEADER_SIZE {
            Err(Error::StackOverflow)
        } else if data.len() > u16::MAX as usize {
            Err(Error::ValueTooLarge)
        } else {
            let data_len = (data.len() as u16).to_be_bytes();
            let data_end = self.usage + data.len();

            self.data[self.usage..data_end].copy_from_slice(data);
            self.data[data_end..data_end + 2].copy_from_slice(&data_len);
            self.data[data_end + 2] = *code;

            self.usage += data.len() + Self::HEADER_SIZE;

            Ok(())
        }
    }

    fn pop(&mut self) -> Option<(ShortID, &[u8])> {
        self.iter_pointer = 0;
        let (id, range) = self.fetch_header_from(self.usage)?;
        self.usage -= Self::HEADER_SIZE + range.len();
        Some((id, &self.data[range]))
    }

    fn get(&self, code: ShortID) -> Option<&[u8]> {
        let mut offset = self.usage;

        while offset > 0 {
            let (id, range) = self.fetch_header_from(offset)?;

            if id == code {
                return Some(&self.data[range]);
            }

            offset = range.start;
        }

        None
    }

    fn iter_reset(&mut self) {
        self.iter_pointer = self.usage;
    }

    fn iter_next(&mut self) -> Option<(ShortID, &[u8])> {
        let (id, range) = self.fetch_header_from(self.iter_pointer)?;

        self.iter_pointer = self
            .iter_pointer
            .saturating_sub(range.len() + Self::HEADER_SIZE);

        Some((id, &self.data[range]))
    }
}

#[cfg(test)]
mod does {
    use super::*;

    const OVERHEAD: usize = FixedSizeStack::<0>::OVERHEAD;

    #[test]
    fn accept_push() {
        let mut stack = FixedSizeStack::<{ OVERHEAD + 1 }>::new();
        stack.push(0.into(), &[42]).unwrap();
    }

    #[test]
    fn accept_pop() {
        let data: &[u8] = &[42];
        let mut stack = FixedSizeStack::<{ OVERHEAD + 1 }>::new();
        stack.push(0.into(), data).unwrap();
        assert_eq!(stack.pop(), Some((0.into(), data)));
    }

    #[test]
    fn accept_get() {
        let mut stack = FixedSizeStack::<{ (OVERHEAD + 1) * 4 }>::new();

        stack.push(0.into(), &[1]).unwrap();
        stack.push(1.into(), &[2]).unwrap();
        stack.push(0.into(), &[3]).unwrap();
        stack.push(1.into(), &[4]).unwrap();
        assert_eq!(stack.get(0.into()).unwrap()[0], 3);

        stack.pop();
        stack.pop();
        assert_eq!(stack.get(0.into()).unwrap()[0], 1);
    }

    #[test]
    fn accept_iter() {
        let data1: &[u8] = &[42, 69];
        let data2: &[u8] = &[5, 57];

        let mut stack = FixedSizeStack::<{ (OVERHEAD + 2) * 2 }>::new();

        stack.push(0.into(), data1).unwrap();
        stack.push(1.into(), data2).unwrap();

        stack.iter_reset();
        assert_eq!(stack.iter_next(), Some((1.into(), data2)));
        assert_eq!(stack.iter_next(), Some((0.into(), data1)));
        assert_eq!(stack.iter_next(), None);
    }
}
