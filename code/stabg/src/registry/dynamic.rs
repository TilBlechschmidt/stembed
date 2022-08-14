use super::*;

pub struct DynamicRegistry(alloc::vec::Vec<Identifier>);

#[cfg(feature = "alloc")]
impl DynamicRegistry {
    pub fn new() -> Self {
        Self(alloc::vec::Vec::new())
    }

    pub fn register(&mut self, id: Identifier) -> ShortID {
        self.lookup(id).unwrap_or_else(|| {
            let code = self.0.len() as ShortID;

            if code >= ShortID::MAX - RESERVED_COUNT {
                panic!("attempted to register more types than supported");
            }

            self.0.push(id);

            code
        })
    }
}

#[cfg(feature = "alloc")]
impl Registry for DynamicRegistry {
    fn lookup(&self, id: Identifier) -> Option<ShortID> {
        self.0
            .iter()
            .enumerate()
            .find(|(_, c)| **c == id)
            .map(|(i, _)| i as ShortID)
    }
}
