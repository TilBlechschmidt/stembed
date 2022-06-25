#![allow(dead_code)]

use core::mem::MaybeUninit;
use core::ops::{Index, IndexMut, Range};

pub struct HistoryBuffer<T, const N: usize> {
    data: [MaybeUninit<T>; N],
    write_at: usize,
    filled: bool,
}

impl<T, const N: usize> HistoryBuffer<T, N> {
    const INIT: MaybeUninit<T> = MaybeUninit::uninit();

    #[inline]
    pub const fn new() -> Self {
        Self {
            data: [Self::INIT; N],
            write_at: 0,
            filled: false,
        }
    }

    /// Returns the current fill level of the buffer.
    #[inline]
    pub fn len(&self) -> usize {
        if self.filled {
            N
        } else {
            self.write_at
        }
    }

    /// Returns the capacity of the buffer, which is the length of the
    /// underlying backing array.
    #[inline]
    pub fn capacity(&self) -> usize {
        N
    }

    /// Writes an element to the buffer, returning the oldest value if the buffer has reached capacity
    pub fn push(&mut self, t: T) -> Option<T> {
        let old_value = if self.filled {
            // Take the old value before we overwrite it.
            Some(unsafe { self.data[self.write_at].assume_init_read() })
        } else {
            None
        };

        self.data[self.write_at] = MaybeUninit::new(t);

        self.write_at += 1;
        if self.write_at == self.capacity() {
            self.write_at = 0;
            self.filled = true;
        }

        old_value
    }

    /// Removes the latest element from the buffer
    pub fn pop(&mut self) -> Option<T> {
        if self.write_at == 0 {
            if self.filled {
                self.filled = false;
                self.write_at = self.capacity() - 1;
                // Takes ownership of the value by copying it out of the array
                Some(unsafe { self.data[self.write_at].assume_init_read() })
            } else {
                None
            }
        } else {
            self.write_at -= 1;
            // Takes ownership of the value by copying it out of the array
            Some(unsafe { self.data[self.write_at].assume_init_read() })
        }
    }

    /// Peeks at the last element in the buffer
    pub fn back(&self) -> Option<&T> {
        self.peek_back(0)
    }

    /// Peeks at the n-th element from the back where an offset of zero returns the latest element
    pub fn peek_back(&self, offset: usize) -> Option<&T> {
        self.index_for_offset(offset)
            .map(|index| unsafe { self.data[index].assume_init_ref() })
    }

    /// Mutable version of [`peek_back`]
    pub fn peek_back_mut(&mut self, offset: usize) -> Option<&mut T> {
        self.index_for_offset(offset)
            .map(|index| unsafe { self.data[index].assume_init_mut() })
    }

    /// Iterates over a range of indices where a zero index is the last element and increasing indices return older elements
    pub fn iter(&self, range: Range<usize>) -> impl Iterator<Item = &T> {
        range.map(|offset| &self[offset])
    }

    /// Calculates the index in the underlying array for the given offset from the back
    /// where an offset of zero indicates the latest element and increasing offsets target older elements.
    fn index_for_offset(&self, mut offset: usize) -> Option<usize> {
        if offset >= self.len() {
            return None;
        }

        // Start with the index of the latest element
        let mut index = if self.write_at == 0 {
            if self.filled {
                self.capacity() - 1
            } else {
                return None;
            }
        } else {
            self.write_at - 1
        };

        // Then continually decrease it, wrapping around
        // TODO This can probably be done in a single instruction with some mod operator
        while offset > 0 {
            if index == 0 {
                index = self.capacity() - 1;
            } else {
                index -= 1;
            }

            offset -= 1;
        }

        Some(index)
    }
}

impl<T, const N: usize> Index<usize> for HistoryBuffer<T, N> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        self.peek_back(index).expect("array index out of bounds")
    }
}

impl<T, const N: usize> IndexMut<usize> for HistoryBuffer<T, N> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        self.peek_back_mut(index)
            .expect("array index out of bounds")
    }
}

/// Creates a new HistoryBuffer with the given elements, just like `vec![]`. Note that it does not verify that the size of the buffer matches the number of passed elements!
#[allow(unused_macros)]
macro_rules! buf {
    ( $( $x:expr ),* ) => {
        {
            let mut temp_vec = crate::HistoryBuffer::new();
            $(
                temp_vec.push($x);
            )*
            temp_vec
        }
    };
}

// "Export" the above macro for other modules to use
// See: https://stackoverflow.com/a/31749071
#[allow(unused_imports)]
pub(crate) use buf;

// This is so precarious that we better write inline tests for it
#[cfg(test)]
mod does {
    use super::HistoryBuffer;

    #[test]
    fn allocate_with_macro() {
        let mut buffer: HistoryBuffer<_, 3> = buf!['a', 'b', 'c'];
        assert_eq!(buffer.len(), 3);
        assert_eq!(buffer.pop(), Some('c'));
        assert_eq!(buffer.pop(), Some('b'));
        assert_eq!(buffer.pop(), Some('a'));
    }

    #[test]
    fn indexes_correct_value() {
        let buffer: HistoryBuffer<_, 3> = buf!['a', 'b', 'c'];
        assert_eq!(buffer[0], 'c');
        assert_eq!(buffer[1], 'b');
        assert_eq!(buffer[2], 'a');
    }

    #[test]
    #[should_panic(expected = "array index out of bounds")]
    fn panic_on_oob_index() {
        let buffer: HistoryBuffer<_, 1> = buf!['a'];
        buffer[2];
    }

    #[test]
    fn peeks_correct_value() {
        let mut buffer = HistoryBuffer::<u8, 2>::new();
        for i in 0..10 {
            buffer.push(i);
            assert_eq!(buffer.back(), Some(&i));
        }
    }

    #[test]
    fn write_at_correct_locations() {
        let mut buffer = HistoryBuffer::<u8, 2>::new();
        assert_eq!(buffer.write_at, 0);
        buffer.push(1);
        assert_eq!(buffer.write_at, 1);
        buffer.push(2);
        assert_eq!(buffer.write_at, 0);
        buffer.push(3);
        assert_eq!(buffer.write_at, 1);
        buffer.push(4);
    }

    #[test]
    fn return_old_values_correctly() {
        let mut buffer = HistoryBuffer::<usize, 2>::new();
        assert_eq!(buffer.push(1), None);
        assert_eq!(buffer.push(2), None);
        for i in 3..100 {
            assert_eq!(buffer.push(i), Some(i - 2));
        }
    }

    #[test]
    fn reset_when_popping_only_value() {
        let mut buffer = HistoryBuffer::<usize, 2>::new();
        assert_eq!(buffer.write_at, 0);
        assert_eq!(buffer.filled, false);

        buffer.push(1);
        assert_eq!(buffer.write_at, 1);
        assert_eq!(buffer.filled, false);

        assert_eq!(buffer.pop(), Some(1));
        assert_eq!(buffer.write_at, 0);
        assert_eq!(buffer.filled, false);
    }

    #[test]
    fn handle_pop_when_filled() {
        let mut buffer = HistoryBuffer::<usize, 2>::new();
        assert_eq!(buffer.write_at, 0);
        assert_eq!(buffer.filled, false);

        buffer.push(1);
        buffer.push(2);
        assert_eq!(buffer.write_at, 0);
        assert_eq!(buffer.filled, true);

        assert_eq!(buffer.pop(), Some(2));
        assert_eq!(buffer.write_at, 1);
        assert_eq!(buffer.filled, false);
    }

    #[test]
    fn report_correct_length() {
        let buffer: HistoryBuffer<char, 2> = buf!['a', 'b'];
        assert_eq!(buffer.len(), 2);
    }
}
