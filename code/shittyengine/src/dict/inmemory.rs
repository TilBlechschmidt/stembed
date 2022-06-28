use super::{find_longest_matching_prefix, Dictionary, OutlineMatch};
use crate::formatter::FormatterCommand;
use alloc::{collections::BTreeMap, string::String, vec::Vec};

pub struct InMemoryDictionary<'c, Stroke> {
    entries: BTreeMap<Vec<Stroke>, Vec<FormatterCommand<String>>>,
    longest_outline: usize,
}

impl<'c, Stroke: Ord> InMemoryDictionary<'c, Stroke> {
    pub fn new() -> Self {
        Self {
            entries: BTreeMap::new(),
            longest_outline: 0,
        }
    }

    pub fn add(
        &mut self,
        outline: Vec<Stroke>,
        commands: Vec<FormatterCommand<String>>,
    ) -> Option<Vec<FormatterCommand<String>>> {
        assert!(
            outline.len() < u8::MAX as usize,
            "only outlines with less than 256 strokes are supported"
        );

        if outline.len() > self.longest_outline {
            self.longest_outline = outline.len();
        }

        self.entries.insert(outline, commands)
    }
}

impl<'c, Stroke: Ord> Default for InMemoryDictionary<'c, Stroke> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'c, Stroke: Ord + Clone> Dictionary for InMemoryDictionary<'c, Stroke> {
    type Stroke = Stroke;
    type StringData = String;
    type CommandIter = alloc::slice::Iter<'c, FormatterCommand<Self::StringData>>;

    fn match_prefix<'s>(
        &'c mut self,
        strokes: impl Iterator<Item = &'s Self::Stroke> + Clone,
    ) -> Option<OutlineMatch<Self::StringData, Self::CommandIter>>
    where
        Self::Stroke: 's,
    {
        find_longest_matching_prefix(self.longest_outline, strokes, |strokes| {
            // This is very inefficient but oh well. It is for debugging purposes only anyways.
            let outline = strokes.cloned().collect::<Vec<_>>();

            self.entries
                .get(&outline)
                .map(|commands| commands.as_slice())
        })
    }
}
