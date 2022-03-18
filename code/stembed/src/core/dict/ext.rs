use super::{super::engine::FetchedOutline, CommandList, Dictionary};
use crate::constants::AVG_OUTLINE_RATIO;
use core::ops::Deref;
use smallvec::SmallVec;

type OutlineList<'s, Stroke, OutputCommand> =
    SmallVec<[FetchedOutline<'s, Stroke, OutputCommand>; AVG_OUTLINE_RATIO]>;

pub struct DictionaryHandler<D: Dictionary>(D);

impl<D, Stroke, OutputCommand> DictionaryHandler<D>
where
    D: Dictionary<Stroke = Stroke, OutputCommand = OutputCommand>,
{
    pub fn new(dictionary: D) -> Self {
        Self(dictionary)
    }

    pub async fn lookup(&self, outline: &[D::Stroke]) -> Option<CommandList<D::OutputCommand>> {
        self.0.lookup(outline).await
    }

    pub async fn find_outlines<'s, 'f>(
        &self,
        strokes: &'s [Stroke],
    ) -> OutlineList<'s, Stroke, OutputCommand> {
        let longest_outline_length = self.0.longest_outline_length();

        // Helper function which finds one outline in the given slice
        let find_longest_matching_outline = |slice: &'s [Stroke]| async move {
            assert!(
                !slice.is_empty(),
                "Attempted to find outline in empty slice!"
            );

            let mut outline_length = slice.len().min(longest_outline_length);

            // Try finding outlines from outline_length to 1
            while outline_length > 0 {
                let outline = &slice[0..outline_length];
                if let Some(commands) = self.lookup(outline).await {
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
                commands: self.0.fallback_commands(&slice[0]),
            }
        };

        // Match all outlines
        let mut offset = 0;
        let mut outlines = SmallVec::new();

        while offset < strokes.len() {
            let outline = find_longest_matching_outline(&strokes[offset..]).await;
            offset += outline.strokes.len();
            outlines.push(outline);
        }

        outlines
    }
}

impl<D, Stroke, OutputCommand> Deref for DictionaryHandler<D>
where
    D: Dictionary<Stroke = Stroke, OutputCommand = OutputCommand>,
{
    type Target = D;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
