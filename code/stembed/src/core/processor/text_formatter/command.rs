use super::{AttachmentMode, CapitalizationMode};
use alloc::string::String;

#[derive(Debug, Clone)]
pub enum TextOutputCommand {
    Write(String),
    ChangeCapitalization(CapitalizationMode),
    ChangeAttachment(AttachmentMode),
    ChangeDelimiter(char),
    ResetFormatting,
}
