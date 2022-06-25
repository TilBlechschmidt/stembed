/// Formatter command containing a reference to a string allocated elsewhere
pub type FormatterCommand<'s> = GenericFormatterCommand<&'s str>;

/// Formatter command owning its heap allocated string contents
#[cfg(feature = "alloc")]
pub type OwnedFormatterCommand = GenericFormatterCommand<alloc::string::String>;

/// Formatter command which is generic over the type of string data it contains
#[derive(Debug, PartialEq, Eq, Clone, Hash, PartialOrd, Ord)]
pub enum GenericFormatterCommand<S> {
    Write(S),
    ChangeCapitalization(CapitalizationMode),
    ChangeAttachment(AttachmentMode),
    ResetFormatting,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash, PartialOrd, Ord)]
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

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash, PartialOrd, Ord)]
pub enum CapitalizationMode {
    /// Retain original capitalization
    Unchanged,
    /// Convert to all `UPPERCASE` (caps lock)
    Uppercase,
    /// Convert to all `lowercase`
    Lowercase,
    /// Uppercase the first letter and lowercase the rest
    Capitalize,
    /// Convert the next word to lowercase, then switch to [`Capitalize`](CapitalizationMode::Capitalize) — useful for e.g. `camelCase`.
    LowerThenCapitalize,

    /// Variant of [`Uppercase`](CapitalizationMode::Uppercase) that only applies to the next word
    UppercaseNext,
    /// Variant of [`Lowercase`](CapitalizationMode::Lowercase) that only applies to the next word
    LowercaseNext,
    /// Variant of [`Capitalize`](CapitalizationMode::Capitalize) that only applies to the next word
    CapitalizeNext,
}
