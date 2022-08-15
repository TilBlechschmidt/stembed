use crate::ShortID;

#[cfg(feature = "alloc")]
mod dynamic;
mod fixed;

#[cfg(feature = "alloc")]
pub use dynamic::DynamicStack;
pub use fixed::FixedSizeStack;

/// Memory overflow errors caused while modifying a [`Stack`](Stack)
#[derive(Debug)]
pub enum StackError {
    /// The underlying storage medium has no space left to store the provided value
    StackOverflow,
    /// Internal constraints prevent a value of this size from being stored (usually `2^16`)
    ValueTooLarge,
}

/// LIFO storage for arbitrary binary data with a type tag
pub trait Stack {
    /// Removes all values from the stack
    fn clear(&mut self);

    /// Appends the given data to the stack and associates the type code with it
    fn push(&mut self, code: ShortID, data: &[u8]) -> Result<(), StackError>;
    /// Removes the last pushed value from the stack permanently
    fn pop(&mut self) -> Option<(ShortID, &[u8])>;
    /// Retrieves the latest pushed value with the given type code, ignores older values
    fn get(&self, code: ShortID) -> Option<&[u8]>;

    /// Resets the internal iteration pointer to the newest value, see [`iter_next`](Stack::iter_next)
    fn iter_reset(&mut self);
    /// Iterates through the stack from newest to oldest without modifying the values.
    /// Due to implementation details, this could not be solved with a true Iterator. If you have any ideas, let me know :D
    // TODO Find a way to return one iterator type which works for all implementations (by holding a ref)
    fn iter_next(&mut self) -> Option<(ShortID, &[u8])>;
}

/// On embedded, we are directly transmuting Rust values into their internal memory representation.
/// Hence, we can statically calculate the stack usage of a processor if we know the sizes of the types it
/// will output and the number of outputs it will make. If anything, this produces a high estimate but that is fine.
#[doc(hidden)]
#[allow(clippy::never_loop)]
pub const fn determine_stack_usage(output_count: usize, output_type_sizes: &[usize]) -> usize {
    if output_count == 0 {
        return 0;
    }

    // for x in 0..output_type_sizes.len()
    let mut x = 0;
    while x < output_type_sizes.len() {
        // for y in x..output_type_sizes.len()
        let mut y = x;
        while y < output_type_sizes.len() {
            if output_type_sizes[y] > output_type_sizes[x] {
                x += 1;
                continue;
            } else {
                y += 1;
            }
        }

        return (FixedSizeStack::<0>::OVERHEAD + output_type_sizes[x]) * output_count;
    }

    0
}

#[cfg(test)]
mod does {
    use crate::{determine_stack_usage, FixedSizeStack};

    #[test]
    fn determine_usage_correctly() {
        assert_eq!(
            10 + FixedSizeStack::<0>::OVERHEAD,
            determine_stack_usage(1, &[1, 5, 2, 9, 3, 10, 7, 8])
        );

        assert_eq!(
            (10 + FixedSizeStack::<0>::OVERHEAD) * 2,
            determine_stack_usage(2, &[1, 5, 2, 9, 3, 10, 7, 8])
        );

        assert_eq!(
            10 + FixedSizeStack::<0>::OVERHEAD,
            determine_stack_usage(1, &[10, 10, 10])
        );

        assert_eq!(0, determine_stack_usage(1, &[]));
        assert_eq!(0, determine_stack_usage(0, &[1, 2, 3]));
    }
}
