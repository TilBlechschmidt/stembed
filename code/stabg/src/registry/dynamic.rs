use super::*;

pub struct DynamicRegistry(alloc::vec::Vec<Identifier>);

#[cfg(feature = "alloc")]
impl DynamicRegistry {
    pub fn new() -> Self {
        Self(alloc::vec::Vec::new())
    }
}

#[cfg(feature = "alloc")]
impl Registry for DynamicRegistry {
    fn register(&mut self, id: Identifier) -> Result<ShortID, RegistryError> {
        if let Some(id) = self.lookup(id) {
            Ok(id)
        } else {
            let code = self.0.len() as ShortID;
            self.0.push(id);
            Ok(code)
        }
    }

    fn lookup(&self, id: Identifier) -> Option<ShortID> {
        self.0
            .iter()
            .enumerate()
            .find(|(_, c)| **c == id)
            .map(|(i, _)| i as ShortID)
    }

    fn reverse_lookup(&self, short: ShortID) -> Option<Identifier> {
        self.0.get(short as usize).cloned()
    }
}
