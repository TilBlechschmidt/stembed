use super::{
    entry::{HistoryEntry, OutlineInformation},
    State,
};

/// Tool to modify the [`State`](super::State) by adding/removing strokes
pub struct StateMutator {
    longest_outline_length: usize,
}

impl StateMutator {
    pub fn new(longest_outline_length: usize) -> Self {
        Self {
            longest_outline_length,
        }
    }

    pub fn add<Stroke, const HISTORY_SIZE: usize>(
        &self,
        stroke: Stroke,
        state: &mut State<Stroke, HISTORY_SIZE>,
    ) {
        state.strokes.push(HistoryEntry::new(stroke));

        // Mark all strokes from the back up to the longest possible outline length as uncommitted.
        let mut uncommitted_count = self.longest_outline_length.min(state.strokes.len());

        // Include any strokes up to the next outline border so future matching does not consume strokes 'bound' by another outline
        while let Some(stroke) = state.stroke_from_back(uncommitted_count - 1) {
            if stroke.outline.is_some() {
                break;
            }

            uncommitted_count += 1;
        }

        state.uncommitted_count = state
            .uncommitted_count
            .max(uncommitted_count)
            .min(state.strokes.len());
    }

    pub fn pop<Stroke, const HISTORY_SIZE: usize>(
        &self,
        state: &mut State<Stroke, HISTORY_SIZE>,
    ) -> Option<OutlineInformation> {
        match state.strokes.pop() {
            Some(stroke) => {
                // Since we removed an item, we have to decrement the uncommitted count by one
                if state.uncommitted_count > 0 {
                    state.uncommitted_count -= 1;
                }

                // If we removed a stroke and it started an outline, it can not be part of another outline started earlier
                // (unless something is really broken). Thus our work here is done and we just return said outline info.
                if let Some(outline) = stroke.outline {
                    return Some(outline);
                }
            }

            // If there is nothing to remove, our work is done here
            None => return None,
        }

        // The removed stroke could be part of an outline started earlier, thus we have to do some extra lifting!
        // We travel backwards to find the most recent outline and then check whether the removed stroke was part of it.
        let mut offset = 0;
        while let Some(stroke) = state.stroke_from_back(offset) {
            if let Some(outline) = stroke.outline.as_ref() {
                // The expected length of the outline is `offset` (number of strokes 'travelled' before encountering the outline boundary)
                // plus one for the stroke we are currently looking at plus one for the stroke we removed.
                let outline_contained_removed_stroke = outline.length as usize == offset + 2;

                if outline_contained_removed_stroke {
                    // Remove the outline
                    let outline = stroke.outline.take();

                    // Adjust the uncommitted_count to indicate that the outline we remove the stroke from needs rematching
                    state.uncommitted_count = state.uncommitted_count.max(offset);

                    // Return the outline we removed
                    return outline;
                } else {
                    // Since it was not part of any stroke, there is nothing left to do.
                    break;
                }
            }

            offset += 1;
        }

        None
    }
}

#[cfg(test)]
mod does {
    use super::super::entry::OutlineInformation;
    use super::*;
    use crate::buf;

    mod when_removing {
        use super::*;

        #[test]
        fn nothing_when_not_part_of_outline() {
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
                uncommitted_count: 0,
            };

            let engine = StateMutator::new(2);

            assert_eq!(engine.pop(&mut state), None);
            assert_eq!(state.uncommitted_count, 0);
        }

        #[test]
        fn nothing_when_start_of_outline() {
            let mut state = State::<char, 3> {
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
                            length: 1,
                            commands: 3
                        }
                    )
                ],
                uncommitted_count: 0,
            };

            let engine = StateMutator::new(2);

            assert_eq!(
                engine.pop(&mut state),
                Some(OutlineInformation {
                    length: 1,
                    commands: 3
                })
            );
            assert_eq!(state.uncommitted_count, 0);
        }

        #[test]
        fn adjust_uncommitted_count() {
            let mut state = State::<char, 3> {
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
                            length: 1,
                            commands: 3
                        }
                    ),
                    HistoryEntry::new('a')
                ],
                uncommitted_count: 2,
            };

            let engine = StateMutator::new(2);

            assert_eq!(engine.pop(&mut state), None);
            assert_eq!(state.uncommitted_count, 1);

            assert_eq!(
                engine.pop(&mut state),
                Some(OutlineInformation {
                    length: 1,
                    commands: 3
                })
            );
            assert_eq!(state.uncommitted_count, 0);
        }
    }

    mod when_adding {
        use super::*;

        #[test]
        fn reset_uncommitted_to_minimum() {
            let mut state = State::<char, 3> {
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
                            length: 1,
                            commands: 0
                        }
                    )
                ],
                uncommitted_count: 0,
            };

            let engine = StateMutator::new(2);
            engine.add('c', &mut state);

            assert_eq!(state.uncommitted_count, 2);
        }

        #[test]
        fn reset_uncommitted_to_history_len() {
            let mut state = State::<char, 2> {
                strokes: buf![HistoryEntry::with_outline(
                    'a',
                    OutlineInformation {
                        length: 1,
                        commands: 0
                    },
                )],
                uncommitted_count: 0,
            };

            let engine = StateMutator::new(4);
            engine.add('b', &mut state);

            assert_eq!(state.uncommitted_count, 2);
        }

        #[test]
        fn reset_to_oldest_outline_past_minimum() {
            let mut state = State::<char, 3> {
                strokes: buf![
                    HistoryEntry::with_outline(
                        '0',
                        OutlineInformation {
                            length: 1,
                            commands: 0
                        }
                    ),
                    HistoryEntry::with_outline(
                        'a',
                        OutlineInformation {
                            length: 2,
                            commands: 0
                        }
                    ),
                    HistoryEntry::new('b')
                ],
                uncommitted_count: 0,
            };

            let engine = StateMutator::new(2);
            engine.add('c', &mut state);

            assert_eq!(state.uncommitted_count, 3);
        }
    }
}
