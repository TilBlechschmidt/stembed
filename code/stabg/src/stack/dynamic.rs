use super::*;
use alloc::vec::Vec;
use core::ops::Range;

// TODO 80% code duplication with the fixed size stack

/// Heap allocation based stack that dynamically grows
///
/// Stores and manages values in the same way as the [`FixedSizeStack`](super::FixedSizeStack)
/// but instead of an array, it uses a [`Vec`](alloc::vec::Vec) as the underlying storage primitive.
/// Note that memory may not be freed immediately when a value is popped from the stack.
#[doc(cfg(feature = "alloc"))]
pub struct DynamicStack {
    data: Vec<u8>,
    usage: usize,
    iter_pointer: usize,
}

impl DynamicStack {
    const HEADER_SIZE: usize = 6;

    pub fn new() -> Self {
        Self {
            data: Vec::new(),
            usage: 0,
            iter_pointer: 0,
        }
    }

    /// Frees unused memory that has not yet been returned to the allocator
    pub fn shrink_to_fit(&mut self) {
        self.data.shrink_to(self.usage);
    }

    /// Fetches the header for the entry located immediately before the given offset
    fn fetch_header_from(&self, offset: usize) -> Option<(ShortID, Range<usize>)> {
        if offset < Self::HEADER_SIZE {
            None
        } else {
            let id: ShortID = u32::from_be_bytes([
                self.data[offset - 4],
                self.data[offset - 3],
                self.data[offset - 2],
                self.data[offset - 1],
            ]);
            let length = u16::from_be_bytes([self.data[offset - 6], self.data[offset - 5]]);
            let range = offset - Self::HEADER_SIZE - length as usize..offset - Self::HEADER_SIZE;
            Some((id, range))
        }
    }
}

impl Default for DynamicStack {
    fn default() -> Self {
        Self::new()
    }
}

impl Stack for DynamicStack {
    fn clear(&mut self) {
        self.usage = 0;
        self.iter_pointer = 0;
        self.data.clear();
    }

    fn push(&mut self, code: ShortID, data: &[u8]) -> Result<(), StackError> {
        if data.len() > u16::MAX as usize {
            Err(StackError::ValueTooLarge)
        } else {
            while self.data.len() < self.usage + data.len() + Self::HEADER_SIZE {
                self.data.push(0);
            }

            let data_len = (data.len() as u16).to_be_bytes();
            let data_end = self.usage + data.len();
            let data_len_end = data_end + 2;
            let code_end = data_len_end + 4;
            let code_bytes = code.to_be_bytes();

            self.data[self.usage..data_end].copy_from_slice(data);
            self.data[data_end..data_len_end].copy_from_slice(&data_len);
            self.data[data_len_end..code_end].copy_from_slice(&code_bytes);

            self.usage += data.len() + Self::HEADER_SIZE;

            self.shrink_to_fit();

            Ok(())
        }
    }

    /// Removes the last pushed value from the stack permanently
    ///
    /// # Memory reclaim
    /// Does not immediately free the associated memory as a reference to it is returned.
    /// Call [`shrink_to_fit`](Self::shrink_to_fit) to reclaim unused space immediately.
    /// This will happen automatically when you push new values.
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
