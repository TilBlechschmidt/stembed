use crate::{
    context::{ProcessorBoundary, ValueSet},
    Identifiable,
};

use super::{Identifier, ShortID};

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
    /// Deallocates the short ID assigned to the given identifier
    fn unregister(&mut self, id: Identifier);

    /// Performs a forward lookup based on the provided identifier
    fn lookup(&self, id: Identifier) -> Option<ShortID>;
    /// Exact opposite to [`lookup`](Registry::lookup)
    fn reverse_lookup(&self, short: ShortID) -> Option<Identifier>;

    /// Returns whether or not a short ID has been assigned to the given identifier
    fn contains(&self, id: Identifier) -> bool {
        self.lookup(id).is_some()
    }
}

pub struct FixedSizeRegistry<const CAPACITY: usize>([Option<Identifier>; CAPACITY]);

impl<const CAPACITY: usize> FixedSizeRegistry<CAPACITY> {
    pub(crate) fn new() -> Self {
        Self([None; CAPACITY])
    }
}

impl<const CAPACITY: usize> Default for FixedSizeRegistry<CAPACITY> {
    fn default() -> Self {
        let mut registry = Self::new();
        registry
            .register(ValueSet::IDENTIFIER)
            .expect("failed to register internal type ValueSet");
        registry
            .register(ProcessorBoundary::IDENTIFIER)
            .expect("failed to register internal type ProcessorBoundary");
        registry
    }
}

impl<const CAPACITY: usize> Registry for FixedSizeRegistry<CAPACITY> {
    fn register(&mut self, id: Identifier) -> Result<ShortID, RegistryError> {
        if let Some(id) = self.lookup(id) {
            Ok(id)
        } else {
            for (i, entry) in self.0.iter_mut().enumerate() {
                if entry.is_none() {
                    *entry = Some(id);
                    return Ok((i as u8).into());
                }
            }

            Err(RegistryError::NoSpaceLeft)
        }
    }

    fn unregister(&mut self, id: Identifier) {
        for entry in self.0.iter_mut() {
            if entry.as_ref() == Some(&id) {
                *entry = None;
            }
        }
    }

    fn lookup(&self, id: Identifier) -> Option<ShortID> {
        for (i, entry) in self.0.iter().enumerate() {
            if entry.as_ref() == Some(&id) {
                return Some((i as u8).into());
            }
        }

        None
    }

    fn reverse_lookup(&self, short: ShortID) -> Option<Identifier> {
        *self.0.get(short as usize)?
    }
}
