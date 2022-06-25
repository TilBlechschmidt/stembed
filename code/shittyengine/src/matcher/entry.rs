use core::ops::Deref;

#[derive(Debug, PartialEq, Eq)]
pub struct OutlineInformation {
    /// Number of strokes in outline
    pub length: u8,

    /// Number of commands associated with outline
    pub commands: u8,
}

#[derive(Debug, PartialEq, Eq)]
pub struct HistoryEntry<Stroke> {
    pub(super) stroke: Stroke,

    /// Information about the outline starting at this stroke, if applicable
    pub(super) outline: Option<OutlineInformation>,
}

impl<Stroke> HistoryEntry<Stroke> {
    pub(super) fn new(stroke: Stroke) -> Self {
        Self {
            stroke,
            outline: None,
        }
    }

    #[allow(dead_code)]
    pub(super) fn with_outline(stroke: Stroke, outline: OutlineInformation) -> Self {
        Self {
            stroke,
            outline: Some(outline),
        }
    }
}

impl<Stroke> Deref for HistoryEntry<Stroke> {
    type Target = Stroke;

    fn deref(&self) -> &Self::Target {
        &self.stroke
    }
}
