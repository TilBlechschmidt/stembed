use super::{entry::OutlineInformation, State};
use core::ops::{Deref, Range};

/// Outline at the end of the history stack which has to be undone before new outlines can be committed
pub struct TrailingOutline<'s, Stroke, const HISTORY_SIZE: usize> {
    state: &'s mut State<Stroke, HISTORY_SIZE>,
    range: Range<usize>,
}

impl<'s, Stroke, const HISTORY_SIZE: usize> TrailingOutline<'s, Stroke, HISTORY_SIZE> {
    /// Consumes the trailing outline, removing it from the associated EngineState.
    /// Usually this function is called when you have undone the side-effects caused when this outline was initially committed.
    pub fn remove(self) {
        self.state.strokes[self.range.end - 1].outline = None;
    }

    /// Information about the outline itself
    pub fn outline(&self) -> &OutlineInformation {
        self.state.strokes[self.range.end - 1]
            .outline
            .as_ref()
            .expect("encountered TrailingOutline without outline")
    }

    /// Strokes contained within the outline
    pub fn strokes(&self) -> impl Iterator<Item = &Stroke> {
        self.range
            .clone()
            .rev()
            .map(|i| &self.state.strokes[i].stroke)
    }
}

impl<'s, Stroke, const HISTORY_SIZE: usize> Deref for TrailingOutline<'s, Stroke, HISTORY_SIZE> {
    type Target = OutlineInformation;

    fn deref(&self) -> &Self::Target {
        self.outline()
    }
}

/// Tool to drive the [`State`](super::State) into a resolved state with all strokes matched
pub struct StateResolver;

impl StateResolver {
    pub fn new() -> Self {
        Self {}
    }

    /// Attempts to commit an outline given by a number of strokes forming a prefix of the uncommitted stroke list of the given state.
    /// You have to handle the returned error, otherwise the resolver will just throw it over and over again.
    pub fn commit<'s, Stroke, const HISTORY_SIZE: usize>(
        &self,
        prefix_length: u8,
        command_count: u8,
        state: &'s mut State<Stroke, HISTORY_SIZE>,
    ) -> Result<(), TrailingOutline<'s, Stroke, HISTORY_SIZE>> {
        // Sanity-check the inputs
        if prefix_length == 0 || state.uncommitted_count < prefix_length as usize {
            panic!("invalid prefix length, not enough uncommitted strokes present to fit")
        }

        // Check if the prefix length matches an already present outline exactly and commit it if applicable.
        // This allows commits that will not change the outline information even with trailing outlines present,
        // optimizing and undo-redo cycle away that would otherwise occur.
        if state
            .stroke_from_back(state.uncommitted_count - 1)
            .and_then(|stroke| stroke.outline.as_ref())
            .map(|outline| outline.length == prefix_length)
            .unwrap_or(false)
        {
            state.uncommitted_count -= prefix_length as usize;
            return Ok(());
        }

        // Bail if we have a trailing outline that has to be removed first
        if let Some(range) = self.find_trailing_outline(state) {
            return Err(TrailingOutline { state, range });
        }

        // Apply the commit by storing the outline information and adjusting the uncommitted count
        if let Some(stroke) = &mut state.stroke_from_back(state.uncommitted_count - 1) {
            stroke.outline = Some(OutlineInformation {
                length: prefix_length,
                commands: command_count,
            });
            state.uncommitted_count -= prefix_length as usize;
            return Ok(());
        }

        panic!("attempted to commit on inconsistent state")
    }

    /// Finds any outlines in the uncommitted section of the engine state. They are usually present when a stroke has been added after a previously committed outline.
    /// Since the stroke might alter the outline, its strokes will be included in the "uncommitted" section of the buffer and resolving new outlines requires the old one to be undone first.
    fn find_trailing_outline<Stroke, const HISTORY_SIZE: usize>(
        &self,
        state: &mut State<Stroke, HISTORY_SIZE>,
    ) -> Option<Range<usize>> {
        for offset in 0..state.uncommitted_count {
            if let Some(outline) = state.strokes[offset].outline.as_ref() {
                let length = outline.length as usize;
                return Some(offset + 1 - length..offset + 1);
            }
        }

        None
    }
}

impl Default for StateResolver {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod does {
    use super::super::entry::HistoryEntry;
    use super::*;
    use crate::buf;
    use alloc::{vec, vec::Vec};

    #[test]
    fn decrement_uncommitted_count() {
        let mut state = State::<char, 3> {
            strokes: buf![
                HistoryEntry::new('a'),
                HistoryEntry::new('b'),
                HistoryEntry::new('c')
            ],
            uncommitted_count: 2,
        };

        assert!(StateResolver::new().commit(2, 0, &mut state).is_ok());
        assert_eq!(state.uncommitted_count, 0);
    }

    #[test]
    fn write_outline() {
        let mut state = State::<char, 3> {
            strokes: buf![
                HistoryEntry::new('a'),
                HistoryEntry::new('b'),
                HistoryEntry::new('c')
            ],
            uncommitted_count: 2,
        };

        assert!(StateResolver::new().commit(2, 0, &mut state).is_ok());
        assert_eq!(
            state
                .stroke_from_back(1)
                .unwrap()
                .outline
                .as_ref()
                .unwrap()
                .length,
            2
        );
    }

    #[test]
    fn ignore_committed_outlines() {
        let mut state = State::<char, 2> {
            strokes: buf![
                HistoryEntry::with_outline(
                    'a',
                    OutlineInformation {
                        length: 1,
                        commands: 0
                    }
                ),
                HistoryEntry::new('b')
            ],
            uncommitted_count: 1,
        };

        assert!(StateResolver::new().commit(1, 0, &mut state).is_ok());
    }

    #[test]
    #[should_panic]
    fn reject_invalid_lengths() {
        let mut state = State::<char, 2> {
            strokes: buf![HistoryEntry::new('a'), HistoryEntry::new('b')],
            uncommitted_count: 2,
        };

        let _ = StateResolver::new().commit(3, 0, &mut state);
    }

    #[test]
    fn strip_removed_outlines() {
        let mut state = State::<char, 2> {
            strokes: buf![
                HistoryEntry::with_outline(
                    'a',
                    OutlineInformation {
                        length: 2,
                        commands: 0
                    }
                ),
                HistoryEntry::new('b')
            ],
            uncommitted_count: 2,
        };

        if let Err(trailing) = StateResolver::new().commit(1, 0, &mut state) {
            trailing.remove();
            assert!(state.stroke_from_back(0).unwrap().outline.is_none());
        } else {
            panic!();
        }
    }

    #[test]
    fn return_correct_trailing_outline() {
        let mut state = State::<char, 4> {
            strokes: buf![
                HistoryEntry::with_outline(
                    'a',
                    OutlineInformation {
                        length: 1,
                        commands: 0
                    }
                ),
                HistoryEntry::with_outline(
                    'b',
                    OutlineInformation {
                        length: 2,
                        commands: 0
                    }
                ),
                HistoryEntry::new('c'),
                HistoryEntry::new('d')
            ],
            uncommitted_count: 4,
        };

        if let Err(trailing) = StateResolver::new().commit(3, 0, &mut state) {
            let actual = trailing.strokes().collect::<Vec<_>>();
            let expected = vec![&'b', &'c'];

            assert_eq!(actual, expected);
        } else {
            panic!();
        }
    }

    #[test]
    fn permit_redundant_commits_with_trailing_outline() {
        let mut state = State::<char, 4> {
            strokes: buf![
                HistoryEntry::with_outline(
                    'a',
                    OutlineInformation {
                        length: 2,
                        commands: 0
                    }
                ),
                HistoryEntry::new('b'),
                HistoryEntry::with_outline(
                    'c',
                    OutlineInformation {
                        length: 1,
                        commands: 0
                    }
                ),
                HistoryEntry::new('d')
            ],
            uncommitted_count: 4,
        };

        let result = StateResolver::new().commit(2, 0, &mut state);
        assert!(result.is_ok());
    }
}
