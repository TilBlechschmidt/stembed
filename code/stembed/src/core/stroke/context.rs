use smallvec::SmallVec;
use smol_str::SmolStr;

#[derive(Debug, PartialEq, Eq)]
pub enum StrokeContextError {
    EmptyExtraKey,
    ReservedTokenUsed,
    DuplicateKey,
}

#[derive(Debug, PartialEq, Eq)]
pub enum StrokeParseError {
    LeftoverCharacters,
    NoSeparator,
}

#[derive(Debug, Clone, Copy)]
pub enum Key {
    Left(char),
    Middle(char),
    Right(char),
    Extra(&'static str),
}

// TODO This is not 100% unicode safe – at the moment it considers unicode scalars as keys, not grapheme clusters
#[derive(Debug, PartialEq, Eq)]
pub struct StrokeContext {
    pub(crate) left: SmolStr,
    pub(crate) middle: SmolStr,
    pub(crate) right: SmolStr,
    pub(crate) extra: SmallVec<[SmolStr; 16]>,
}

impl StrokeContext {
    pub fn new(
        left: impl AsRef<str>,
        middle: impl AsRef<str>,
        right: impl AsRef<str>,
        extra: &[&str],
    ) -> Result<Self, StrokeContextError> {
        let instance = Self {
            left: SmolStr::new_inline(left.as_ref()),
            middle: SmolStr::new_inline(middle.as_ref()),
            right: SmolStr::new_inline(right.as_ref()),
            extra: extra
                .iter()
                .map(|s| SmolStr::new_inline(s.as_ref()))
                .collect(),
        };

        // Check that no reserved tokens are used in left,middle,right
        for token in ['|', '-'] {
            if instance.left.contains(token)
                || instance.middle.contains(token)
                || instance.right.contains(token)
            {
                return Err(StrokeContextError::ReservedTokenUsed);
            }
        }

        // Ensure that extra keys are not empty or use reserved tokens
        for key in instance.extra.iter() {
            if key.is_empty() {
                return Err(StrokeContextError::EmptyExtraKey);
            } else if key.contains(',') {
                return Err(StrokeContextError::ReservedTokenUsed);
            }
        }

        // Assert that there are no duplicate keys creating ambiguity
        for side in [&instance.left, &instance.middle, &instance.right] {
            for (index, char) in side.as_str().char_indices() {
                if let Some(remainder) = side.as_str().get(index..) {
                    for following_char in remainder.chars().skip(1) {
                        if char == following_char {
                            return Err(StrokeContextError::DuplicateKey);
                        }
                    }
                }
            }
        }

        for index in 0..instance.extra.len() {
            let value = &instance.extra[index];
            let remainder = &instance.extra[(index + 1)..];
            if remainder.contains(value) {
                return Err(StrokeContextError::DuplicateKey);
            }
        }

        Ok(instance)
    }

    pub fn key_count(&self) -> usize {
        self.left.chars().count()
            + self.middle.chars().count()
            + self.right.chars().count()
            + self.extra.len()
    }

    pub fn byte_count(&self) -> usize {
        // Integer divide and ceil to get the minimum byte count required
        // TODO Replace this once https://github.com/rust-lang/rust/issues/88581 lands on stable
        (self.key_count() + 8 - 1) / 8
    }

    pub(crate) fn bit_index(&self, key: &Key) -> Option<usize> {
        match key {
            Key::Left(expected) => find_char_index(&self.left, expected),
            Key::Middle(expected) => {
                find_char_index(&self.middle, expected).map(|i| i + self.left.chars().count())
            }
            Key::Right(expected) => find_char_index(&self.right, expected)
                .map(|i| i + self.left.chars().count() + self.middle.chars().count()),
            Key::Extra(expected) => {
                self.extra
                    .iter()
                    .enumerate()
                    .find(|s| s.1 == expected)
                    .map(|(i, _)| {
                        i + self.left.chars().count()
                            + self.middle.chars().count()
                            + self.right.chars().count()
                    })
            }
        }
    }
}

impl core::fmt::Display for StrokeParseError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            StrokeParseError::LeftoverCharacters => {
                f.write_str("encountered leftover characters at the end of the stroke")
            }
            StrokeParseError::NoSeparator => {
                f.write_str("stroke is missing a separator or 'middle' key to be unambiguous")
            }
        }
    }
}

/// Finds the first occurence of the expected character in the input string and returns the character index (not the byte index)
fn find_char_index(string: &str, expected: &char) -> Option<usize> {
    if string.contains(*expected) {
        Some(string.chars().take_while(|c| c != expected).count())
    } else {
        None
    }
}
