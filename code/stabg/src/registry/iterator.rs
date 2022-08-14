use super::*;

pub struct IteratorRegistry<I>(pub I)
where
    I: Iterator<Item = &'static Identifier> + Clone;

impl<I> Registry for IteratorRegistry<I>
where
    I: Iterator<Item = &'static Identifier> + Clone,
{
    fn lookup(&self, id: Identifier) -> Option<ShortID> {
        // Compress identifiers by deduplicating the iterator before calling `.find`
        // That way we don't have unused identifiers where types are present twice
        let is_first =
            |(i, x): &(usize, &Identifier)| self.0.clone().take(*i).find(|x2| x2 == x).is_none();

        self.0
            .clone()
            .enumerate()
            .filter(is_first)
            .find(|(_, x)| *x == &id)
            .map(|(i, _)| i as ShortID)
    }
}
