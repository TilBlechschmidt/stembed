//! Storage for outlines and their respective commands

use crate::formatter::FormatterCommand;
use core::iter::Take;

#[cfg(feature = "alloc")]
mod inmemory;
#[cfg(feature = "alloc")]
pub use inmemory::InMemoryDictionary;

mod tree;
pub use tree::RadixTreeDictionary;

#[derive(Debug, PartialEq, Eq)]
pub struct OutlineMatch<'c> {
    pub stroke_count: u8,
    pub commands: &'c [FormatterCommand<'c>],
}

pub trait Dictionary<'d> {
    type Stroke;

    fn match_prefix<'s>(
        &'d mut self,
        strokes: impl Iterator<Item = &'s Self::Stroke> + Clone,
    ) -> Option<OutlineMatch<'d>>
    where
        Self::Stroke: 's;
}

/// Helper function for building dictionaries relying on collections indexed by outlines
///
/// Queries the provided data source for prefixes of decreasing length until the longest matching prefix is found.
pub fn find_longest_matching_prefix<'s, 'c, Stroke, Iter, F>(
    longest_outline: usize,
    strokes: Iter,
    mut data_source: F,
) -> Option<OutlineMatch<'c>>
where
    Iter: Iterator<Item = &'s Stroke> + Clone,
    Stroke: 's,
    F: FnMut(Take<Iter>) -> Option<&'c [FormatterCommand<'c>]>,
{
    for i in 0..longest_outline {
        let prefix_length = longest_outline - i;
        let iterator = strokes.clone().take(prefix_length);

        if let Some(commands) = data_source(iterator) {
            return Some(OutlineMatch {
                stroke_count: prefix_length as u8,
                commands,
            });
        }
    }

    None
}

#[cfg(test)]
mod helper_does {
    use super::*;
    use crate::formatter::FormatterCommand;
    use alloc::{vec, vec::Vec};

    #[test]
    fn query_in_right_order() {
        let strokes = [0, 1, 2, 3];
        let longest_outline = 3;

        let mut expected = vec![vec![0], vec![0, 1], vec![0, 1, 2]];

        let result = find_longest_matching_prefix(longest_outline, strokes.iter(), |strokes| {
            let strokes = strokes.cloned().collect::<Vec<_>>();
            assert_eq!(strokes, expected.pop().unwrap());
            None
        });

        assert!(result.is_none());
    }

    #[test]
    fn return_correct_match() {
        let commands = [FormatterCommand::ResetFormatting];
        let mut counter = 0;

        let result = find_longest_matching_prefix(3, [0, 1, 2].iter(), |_| {
            if counter == 1 {
                Some(&commands)
            } else {
                counter += 1;
                None
            }
        });

        assert_eq!(
            result.unwrap(),
            OutlineMatch {
                stroke_count: 2,
                commands: &commands
            }
        );
    }

    #[test]
    fn short_circuit_with_no_outline_length() {
        let result = find_longest_matching_prefix(0, [0, 1, 2].iter(), |_| panic!());
        assert!(result.is_none());
    }
}
