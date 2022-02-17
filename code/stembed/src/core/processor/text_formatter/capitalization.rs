use alloc::string::String;

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum CapitalizationMode {
    /// Retain original capitalization
    None,
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

impl CapitalizationMode {
    pub(super) fn tick(&mut self) {
        use CapitalizationMode::*;

        match *self {
            CapitalizeNext | LowercaseNext | UppercaseNext => *self = CapitalizationMode::None,
            _ => {}
        }
    }

    pub(super) fn change_to(&mut self, new: Self) {
        *self = new;
    }

    pub(super) fn apply(&self, string: String) -> String {
        use CapitalizationMode::*;

        match self {
            CapitalizationMode::None => string,
            Uppercase | UppercaseNext => string.to_uppercase(),
            Lowercase | LowercaseNext | LowerThenCapitalize => string.to_lowercase(),
            Capitalize | CapitalizeNext => capitalize(&string),
        }
    }
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}
