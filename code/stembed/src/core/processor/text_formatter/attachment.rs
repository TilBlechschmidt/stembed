use alloc::string::String;

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum AttachmentMode {
    /// Place the configured delimiter between words
    Delimited,
    /// Intermediate state which, when followed by another "switch" to [`Glue`](AttachmentMode::Glue) mode, will transform into [`Next`](AttachmentMode::Next).
    /// Alternatively reverts back into [`Delimited`](AttachmentMode::Delimited) without any effect if followed by a word.
    ///
    /// TL;DR If two words follow each other with glue "on both sides", they get attached.
    Glue,
    /// Attach the following word and revert back to [`Delimited`](AttachmentMode::Delimited)
    // TODO Add variant of Next which carries capitalization (does not tick it) and applies orthographic rules when applied (for prefixes/suffixes)
    Next,
    /// Never delimit words until the mode is changed through a command
    Always,
}

impl AttachmentMode {
    pub(super) fn tick(&mut self) {
        use AttachmentMode::*;

        match self {
            Glue | Next => *self = Delimited,
            _ => {}
        }
    }

    pub(super) fn change_to(&mut self, new: Self) {
        use AttachmentMode::*;

        if *self == Glue && new == Glue {
            *self = Next;
        } else {
            *self = new;
        }
    }

    pub(super) fn apply(&self, string: String, delimiter: char) -> String {
        use AttachmentMode::*;

        match self {
            Delimited | Glue => format!("{}{}", delimiter, string),
            Next | Always => string,
        }
    }
}
