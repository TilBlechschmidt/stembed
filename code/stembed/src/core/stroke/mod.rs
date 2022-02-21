use crate::{constants::AVG_STROKE_BIT_COUNT, input::InputKeyState};
use core::{fmt::Display, iter::Peekable};
use smallvec::SmallVec;
use smol_str::SmolStr;

mod context;
pub use context::*;

/// Stenography stroke implementation based on a bit vector.
/// Because the bits themselves do not contain any information on what
/// keys they represent, the struct holds a reference to the [`StrokeContext`]
/// that was used to construct it. This will be used when using e.g. `.to_string()`.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Stroke<'c> {
    pub(crate) bit_vec: SmallVec<[u8; AVG_STROKE_BIT_COUNT / 8]>,
    pub(crate) context: &'c StrokeContext,
}

impl<'c> Stroke<'c> {
    fn new(bits: impl Iterator<Item = bool>, context: &'c StrokeContext) -> Self {
        let mut bits = bits.peekable();
        let mut bit_vec = SmallVec::new();

        while bits.peek().is_some() && bit_vec.len() < context.byte_count() {
            let byte = bits
                .by_ref()
                .take(8)
                .enumerate()
                .fold(0u8, |acc, (i, bit)| acc | ((bit as u8) << (7 - i)));

            bit_vec.push(byte);
        }

        // Pad the vector with empty bytes if the input does not contain a sufficient amount of bits
        while bit_vec.len() < context.byte_count() {
            bit_vec.push(0);
        }

        Self { bit_vec, context }
    }

    fn new_empty(context: &'c StrokeContext) -> Self {
        Self {
            bit_vec: core::iter::repeat(0).take(context.byte_count()).collect(),
            context,
        }
    }

    fn bits(&self) -> impl Iterator<Item = bool> + '_ + Clone {
        self.bit_vec
            .iter()
            .flat_map(|byte| (0u8..8).map(move |i| byte & (1 << (7 - i))))
            .map(|bit| bit > 0)
    }

    fn set_bit(&mut self, key: &Key) -> bool {
        match self.context.bit_index(key) {
            None => false,
            Some(index) => {
                let byte_index = index / 8;
                let bit_index = index % 8;
                self.bit_vec[byte_index] |= 1 << (7 - bit_index);
                true
            }
        }
    }

    /// Note that this is a potentially lossy conversion – keys that exist in the mapped input but not in the context will be dropped.
    /// Similarly, keys defined in the context but not present in the mapped input will have their state defaulted to "not pressed".
    pub fn from_input<const KEY_COUNT: usize>(
        input: [InputKeyState; KEY_COUNT],
        keymap: &[Key; KEY_COUNT],
        context: &'c StrokeContext,
    ) -> Self {
        let mut stroke = Stroke::new_empty(context);

        for (pressed, key) in input.into_iter().zip(keymap) {
            if pressed {
                stroke.set_bit(key);
            }
        }

        stroke
    }

    pub fn from_str(
        input: impl AsRef<str>,
        context: &'c StrokeContext,
    ) -> Result<Stroke, StrokeParseError> {
        let input = input.as_ref();
        let mut main_keys = input.chars().take_while(|c| *c != '|').peekable();
        let extra_keys = input
            .find('|')
            .map(|index| input[index + 1..].split(',').peekable());

        // Define the output
        let mut bits: SmallVec<[bool; AVG_STROKE_BIT_COUNT]> = SmallVec::new();

        // Create a couple of helpers
        let scan = |keys: &mut Peekable<_>, expected: &SmolStr, output: &mut SmallVec<_>| -> bool {
            expected.chars().fold(false, |acc, char| {
                let present = if keys.peek() == Some(&char) {
                    keys.next();
                    true
                } else {
                    false
                };

                output.push(present);

                acc | present
            })
        };

        let consume_separator = |keys: &mut Peekable<_>| {
            if keys.peek() == Some(&'-') {
                keys.next();
                true
            } else {
                false
            }
        };

        // Go through left, middle/separator, right
        scan(&mut main_keys, &context.left, &mut bits);
        let contains_middle_keys = scan(&mut main_keys, &context.middle, &mut bits);
        let contains_separator = consume_separator(&mut main_keys);
        let contains_right_keys = scan(&mut main_keys, &context.right, &mut bits);

        // Do some sanity checks
        if contains_right_keys && !contains_middle_keys && !contains_separator {
            return Err(StrokeParseError::NoSeparator);
        } else if main_keys.next().is_some() {
            return Err(StrokeParseError::LeftoverCharacters);
        }

        // Consume extra keys with a little bit of special handling (because it is string sequences not char sequences)
        if let Some(mut extra_keys) = extra_keys {
            context.extra.iter().for_each(|expected| {
                let present = if extra_keys.peek() == Some(&expected.as_str()) {
                    extra_keys.next();
                    true
                } else {
                    false
                };

                bits.push(present);
            })
        }

        // Convert the boolean vector into a stroke
        Ok(Stroke::new(bits.into_iter(), context.clone()))
    }
}

impl<'c> Display for Stroke<'c> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        use core::fmt::Write;

        let mut bits = self.bits();

        // 1. Go through bits for each left key
        let left_keys = self
            .context
            .left
            .chars()
            .zip(bits.by_ref())
            .filter_map(|(char, bit)| if bit { Some(char) } else { None });

        for c in left_keys {
            f.write_char(c)?;
        }

        // 2. Go through bits for each middle key
        let mut middle_keys = self
            .context
            .middle
            .chars()
            .zip(bits.by_ref())
            .filter_map(|(char, bit)| if bit { Some(char) } else { None })
            .peekable();

        let contains_middle_keys = middle_keys.peek().is_some();
        for c in middle_keys {
            f.write_char(c)?;
        }

        // 3. Go through bits for each right key
        let mut right_keys = self
            .context
            .right
            .chars()
            .zip(bits.by_ref())
            .filter_map(|(char, bit)| if bit { Some(char) } else { None })
            .peekable();

        // 3.1 Insert a '-' if we have right keys but no middle keys
        if right_keys.peek().is_some() && !contains_middle_keys {
            f.write_char('-')?;
        }

        // 3.2 Write the right chars
        for c in right_keys {
            f.write_char(c)?;
        }

        // 4. Go through bits for each extra key
        let mut extra_keys = self
            .context
            .extra
            .iter()
            .zip(bits.by_ref())
            .filter_map(|(char, bit)| if bit { Some(char) } else { None })
            .peekable();

        // 4.1 Insert a '|' if we have extra keys
        if extra_keys.peek().is_some() {
            f.write_char('|')?;
        }

        // 4.2 Write the extra keys
        for (i, key) in extra_keys.enumerate() {
            if i > 0 {
                f.write_char(',')?;
            }

            f.write_str(key.as_str())?;
        }

        Ok(())
    }
}
