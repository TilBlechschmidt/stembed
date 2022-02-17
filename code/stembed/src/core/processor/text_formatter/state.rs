use super::{AttachmentMode, CapitalizationMode};
use alloc::string::String;

#[derive(Debug, PartialEq, Eq, Clone)]
pub(super) struct TextFormatterState {
    pub delimiter: char,
    pub attachment: AttachmentMode,
    pub capitalization: CapitalizationMode,
}

impl TextFormatterState {
    pub(super) fn tick(&mut self) {
        self.attachment.tick();
        self.capitalization.tick();
    }

    pub(super) fn apply(&self, string: String) -> String {
        self.attachment
            .apply(self.capitalization.apply(string), self.delimiter)
    }
}

impl Default for TextFormatterState {
    fn default() -> Self {
        Self {
            delimiter: ' ',
            attachment: AttachmentMode::Delimited,
            capitalization: CapitalizationMode::None,
        }
    }
}
