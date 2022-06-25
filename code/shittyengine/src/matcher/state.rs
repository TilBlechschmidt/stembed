use super::entry::HistoryEntry;
use crate::HistoryBuffer;

/// Stroke history defining the state of our state machine
pub struct State<Stroke, const HISTORY_SIZE: usize> {
    pub(super) strokes: HistoryBuffer<HistoryEntry<Stroke>, HISTORY_SIZE>,
    pub(super) uncommitted_count: usize,
}

impl<Stroke, const HISTORY_SIZE: usize> State<Stroke, HISTORY_SIZE> {
    pub fn new() -> Self {
        Self {
            strokes: HistoryBuffer::new(),
            uncommitted_count: 0,
        }
    }

    /// Returns an iterator over the uncommited strokes
    // No outline information is exposed because it may be invalid
    pub fn uncommitted_strokes(&mut self) -> impl Iterator<Item = &Stroke> {
        (0..self.uncommitted_count)
            .rev()
            .map(|i| &self.strokes[i].stroke)
    }

    /// Returns a slice of committed strokes
    pub fn committed_strokes(&mut self) -> impl Iterator<Item = &HistoryEntry<Stroke>> {
        (self.uncommitted_count..self.strokes.len())
            .rev()
            .map(|i| &self.strokes[i])
    }

    /// Returns the `offset`th stroke from the back of history where an offset of zero returnst the newest stroke.
    /// Note that the outline information present might not be accurate if the stroke is uncommitted.
    pub(super) fn stroke_from_back(&mut self, offset: usize) -> Option<&mut HistoryEntry<Stroke>> {
        self.strokes.peek_back_mut(offset)
    }
}

#[cfg(test)]
mod does {
    use super::*;
    use crate::buf;
    use alloc::{vec, vec::Vec};

    #[test]
    fn list_correct_from_back() {
        let mut state = State::<char, 4> {
            strokes: buf![
                HistoryEntry::new('a'),
                HistoryEntry::new('b'),
                HistoryEntry::new('c'),
                HistoryEntry::new('d')
            ],
            uncommitted_count: 4,
        };

        assert_eq!(state.stroke_from_back(0).unwrap(), &HistoryEntry::new('d'));
        assert_eq!(state.stroke_from_back(1).unwrap(), &HistoryEntry::new('c'));
        assert_eq!(state.stroke_from_back(2).unwrap(), &HistoryEntry::new('b'));
        assert_eq!(state.stroke_from_back(3).unwrap(), &HistoryEntry::new('a'));
    }

    #[test]
    fn lists_correct_uncommitted() {
        let mut state = State::<char, 4> {
            strokes: buf![
                HistoryEntry::new('a'),
                HistoryEntry::new('b'),
                HistoryEntry::new('c'),
                HistoryEntry::new('d')
            ],
            uncommitted_count: 4,
        };

        assert_eq!(
            state.uncommitted_strokes().collect::<Vec<_>>(),
            vec![&'a', &'b', &'c', &'d',]
        );

        state.uncommitted_count = 2;
        assert_eq!(
            state.uncommitted_strokes().collect::<Vec<_>>(),
            vec![&'c', &'d']
        );

        state.uncommitted_count = 0;
        assert!(state.uncommitted_strokes().next().is_none());
    }

    #[test]
    fn lists_correct_committed() {
        let mut state = State::<char, 4> {
            strokes: buf![
                HistoryEntry::new('a'),
                HistoryEntry::new('b'),
                HistoryEntry::new('c'),
                HistoryEntry::new('d')
            ],
            uncommitted_count: 4,
        };

        assert!(state.committed_strokes().next().is_none());

        state.uncommitted_count = 2;
        assert_eq!(
            state.committed_strokes().collect::<Vec<_>>(),
            vec![&HistoryEntry::new('a'), &HistoryEntry::new('b')]
        );

        state.uncommitted_count = 0;
        assert_eq!(
            state.committed_strokes().collect::<Vec<_>>(),
            vec![
                &HistoryEntry::new('a'),
                &HistoryEntry::new('b'),
                &HistoryEntry::new('c'),
                &HistoryEntry::new('d'),
            ]
        );
    }
}
