use super::*;

pub struct FixedSizeRegistry<const CAPACITY: usize>([Option<Identifier>; CAPACITY]);

impl<const CAPACITY: usize> FixedSizeRegistry<CAPACITY> {
    pub fn new() -> Self {
        Self([None; CAPACITY])
    }
}

impl<const CAPACITY: usize> Registry for FixedSizeRegistry<CAPACITY> {
    fn register(&mut self, id: Identifier) -> Result<ShortID, RegistryError> {
        if let Some(id) = self.lookup(id) {
            Ok(id)
        } else {
            for (i, entry) in self.0.iter_mut().enumerate() {
                if entry.is_none() && i < ShortID::MAX as usize - RESERVED_COUNT {
                    *entry = Some(id);
                    return Ok((i as u8).into());
                }
            }

            Err(RegistryError::NoSpaceLeft)
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
