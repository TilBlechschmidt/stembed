use super::{Identifier, ShortID};

const RESERVED_COUNT: usize = 2;
pub(crate) const ID_VALUE_SET: ShortID = ShortID::MAX - 0;
pub(crate) const ID_PROC_MARK: ShortID = ShortID::MAX - 1;

#[cfg(feature = "alloc")]
mod dynamic;
mod fixed;

#[cfg(feature = "alloc")]
pub use dynamic::DynamicRegistry;
pub use fixed::FixedSizeRegistry;

/// Errors caused while storing values in the registry
#[derive(Debug)]
pub enum RegistryError {
    /// The underlying storage provider has no slots left
    NoSpaceLeft,
}

/// Stores bidirectional links between [`Identifier`](Identifier)s and [`ShortID`](ShortID)s
pub trait Registry {
    /// Allocates a new short ID for the given identifier. May return a previously allocated one if the identifier is equivalent.
    fn register(&mut self, id: Identifier) -> Result<ShortID, RegistryError>;

    /// Performs a forward lookup based on the provided identifier
    fn lookup(&self, id: Identifier) -> Option<ShortID>;
    /// Exact opposite to [`lookup`](Registry::lookup)
    fn reverse_lookup(&self, short: ShortID) -> Option<Identifier>;

    /// Returns whether or not a short ID has been assigned to the given identifier
    fn contains(&self, id: Identifier) -> bool {
        self.lookup(id).is_some()
    }
}
