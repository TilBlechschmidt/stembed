use super::{Identifier, ShortID};

const RESERVED_COUNT: u32 = 2;
pub(crate) const ID_VALUE_SET: ShortID = ShortID::MAX - 0;
pub(crate) const ID_PROC_MARK: ShortID = ShortID::MAX - 1;

#[cfg(feature = "alloc")]
mod dynamic;
mod iterator;

#[cfg(feature = "alloc")]
pub use dynamic::DynamicRegistry;
pub use iterator::IteratorRegistry;

/// Stores bidirectional links between [`Identifier`](Identifier)s and [`ShortID`](ShortID)s
pub trait Registry {
    /// Performs a forward lookup based on the provided identifier
    fn lookup(&self, id: Identifier) -> Option<ShortID>;

    /// Returns whether or not a short ID has been assigned to the given identifier
    fn contains(&self, id: Identifier) -> bool {
        self.lookup(id).is_some()
    }
}
