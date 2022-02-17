use super::Command;
use crate::constants::{AVG_CMD_COUNT, AVG_STROKE_COUNT};
use smallvec::{SmallVec};

/// Combination of strokes which have been identified as an outline.
/// Additionally contains information about the number of commands the outline produced
/// and whether it can be undone.
#[derive(Debug)]
pub struct MatchedOutline<Stroke> {
    pub strokes: SmallVec<[Stroke; AVG_STROKE_COUNT]>,
    pub command_count: u16,
}

impl<Stroke> MatchedOutline<Stroke>
where
    Stroke: Clone,
{
    pub(super) fn new(strokes: &[Stroke], command_count: usize) -> Self {
        Self {
            strokes: strokes.iter().cloned().collect(),
            command_count: command_count as u16,
        }
    }
}

#[derive(Debug)]
pub struct FetchedOutline<'s, Stroke, OutputCommand> {
    pub strokes: &'s [Stroke],
    pub commands: SmallVec<[Command<OutputCommand>; AVG_CMD_COUNT]>,
}
