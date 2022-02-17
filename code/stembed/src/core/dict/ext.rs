use super::{super::engine::FetchedOutline, Dictionary};
use crate::constants::AVG_OUTLINE_RATIO;
use smallvec::SmallVec;

type OutlineList<'s, Stroke, OutputCommand> =
    SmallVec<[FetchedOutline<'s, Stroke, OutputCommand>; AVG_OUTLINE_RATIO]>;

pub trait DictExt: Dictionary {
    fn find_outlines<'s, 'f>(
        &mut self,
        strokes: &'s [Self::Stroke],
    ) -> OutlineList<'s, Self::Stroke, Self::OutputCommand>;
}

impl<D, Stroke, OutputCommand> DictExt for D
where
    D: Dictionary<Stroke = Stroke, OutputCommand = OutputCommand>,
{
    fn find_outlines<'s, 'f>(
        &mut self,
        strokes: &'s [Stroke],
    ) -> OutlineList<'s, Stroke, OutputCommand> {
        let longest_outline_length = self.longest_outline_length();

        // Helper function which finds one outline in the given slice
        let mut find_longest_matching_outline =
            |slice: &'s [Stroke]| -> FetchedOutline<'s, Stroke, OutputCommand> {
                assert!(
                    !slice.is_empty(),
                    "Attempted to find outline in empty slice!"
                );

                let mut outline_length = slice.len().min(longest_outline_length);

                // Try finding outlines from outline_length to 1
                while outline_length > 0 {
                    let outline = &slice[0..outline_length];
                    if let Some(commands) = self.lookup(outline) {
                        return FetchedOutline {
                            strokes: outline,
                            commands,
                        };
                    }
                    outline_length -= 1;
                }

                // Use the fallback if we do not find any
                FetchedOutline {
                    strokes: &slice[0..1],
                    commands: self.fallback_commands(&slice[0]),
                }
            };

        // Match all outlines
        let mut offset = 0;
        let mut outlines = SmallVec::new();

        while offset < strokes.len() {
            let outline = find_longest_matching_outline(&strokes[offset..]);
            offset += outline.strokes.len();
            outlines.push(outline);
        }

        outlines
    }
}
