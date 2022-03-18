use core::mem::MaybeUninit;

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
            // unsafe { ptr::drop_in_place(self.data[self.write_at].as_mut_ptr()) }
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

    pub fn back(&self) -> Option<&T> {
        if self.write_at == 0 {
            if self.filled {
                Some(unsafe { self.data[self.capacity() - 1].assume_init_ref() })
            } else {
                None
            }
        } else {
            Some(unsafe { self.data[self.write_at - 1].assume_init_ref() })
        }
    }
}

// This is so precarious that we better write inline tests for it
#[cfg(test)]
mod does {
    use super::HistoryBuffer;

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
        for i in 3..1_000_000 {
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
}
