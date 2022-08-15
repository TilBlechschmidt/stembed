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
