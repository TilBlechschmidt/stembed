use super::{AttachmentMode, CapitalizationMode};

#[derive(Clone)]
pub(super) struct TextFormatterState {
    pub(super) attachment: AttachmentMode,
    pub(super) capitalization: CapitalizationMode,
}

impl TextFormatterState {
    pub(super) fn tick(&mut self) {
        self.attachment.tick();
        self.capitalization.tick();
    }

    pub(super) fn apply<'a>(&self, string: &'a str) -> impl Iterator<Item = char> + Clone + 'a {
        self.attachment
            .apply(self.capitalization.apply(string), ' ')
    }
}

impl Default for TextFormatterState {
    fn default() -> Self {
        Self {
            attachment: AttachmentMode::Next,
            capitalization: CapitalizationMode::CapitalizeNext,
        }
    }
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

    pub(super) fn apply(
        &self,
        string: impl Iterator<Item = char> + Clone,
        delimiter: char,
    ) -> impl Iterator<Item = char> + Clone {
        use AttachmentMode::*;

        let delimiter_count = match self {
            Delimited | Glue => 1,
            Next | Always => 0,
        };
        let delimiter = core::iter::repeat(delimiter).take(delimiter_count);

        delimiter.chain(string)
    }
}

impl CapitalizationMode {
    pub(super) fn tick(&mut self) {
        use CapitalizationMode::*;

        match *self {
            CapitalizeNext | LowercaseNext | UppercaseNext => self.change_to(Unchanged),
            LowerThenCapitalize => self.change_to(Capitalize),
            _ => {}
        }
    }

    pub(super) fn change_to(&mut self, new: Self) {
        *self = new;
    }

    pub(super) fn apply<'a>(
        &self,
        string: &'a str,
    ) -> impl Iterator<Item = char> + DoubleEndedIterator + Clone + 'a {
        use CapitalizationMode::*;

        let capitalization_mode = *self;
        string
            .char_indices()
            .map(move |(i, c)| match capitalization_mode {
                Unchanged => c,
                Uppercase | UppercaseNext => c.to_ascii_uppercase(),
                Lowercase | LowercaseNext | LowerThenCapitalize => c.to_ascii_lowercase(),
                Capitalize | CapitalizeNext => {
                    if i == 0 {
                        c.to_ascii_uppercase()
                    } else {
                        c.to_ascii_lowercase()
                    }
                }
            })
    }
}

#[cfg(test)]
mod does {
    use super::*;
    use alloc::{format, string::String};

    #[test]
    fn attach_correctly() {
        let input = "hello";
        let delimiter = '-';

        for attachment in [AttachmentMode::Next, AttachmentMode::Always] {
            let output = attachment
                .apply(input.chars(), delimiter)
                .collect::<String>();

            assert_eq!(output, input);
        }
    }

    #[test]
    fn delimit_correctly() {
        let input = "hello";
        let delimiter = '-';

        for attachment in [AttachmentMode::Delimited, AttachmentMode::Glue] {
            let output = attachment
                .apply(input.chars(), delimiter)
                .collect::<String>();

            assert_eq!(output, format!("{delimiter}{input}"));
        }
    }

    #[test]
    fn capitalize_correctly() {
        let input = "hElLo";

        for capitalization in [
            CapitalizationMode::Uppercase,
            CapitalizationMode::UppercaseNext,
        ] {
            let output = capitalization.apply(input).collect::<String>();
            assert_eq!(output, "HELLO");
        }

        for capitalization in [
            CapitalizationMode::Lowercase,
            CapitalizationMode::LowercaseNext,
            CapitalizationMode::LowerThenCapitalize,
        ] {
            let output = capitalization.apply(input).collect::<String>();
            assert_eq!(output, "hello");
        }

        for capitalization in [
            CapitalizationMode::Capitalize,
            CapitalizationMode::CapitalizeNext,
        ] {
            let output = capitalization.apply(input).collect::<String>();
            assert_eq!(output, "Hello");
        }

        let output = CapitalizationMode::Unchanged
            .apply(input)
            .collect::<String>();
        assert_eq!(output, input);
    }
}
