use core::fmt::{Display, Write};

#[rustfmt::skip]
const KEYMAP: [char; 23] = [
    '#', 'S', 'T', 'K', 'P', 'W', 'H', 'R',
    'A', 'O', '*', 'E', 'U',
    'F', 'R', 'P', 'B', 'L', 'G', 'T', 'S', 'D', 'Z',
];

/// Sum of multiple key-presses which occurred simultaneously.
///
/// Bound to English stenography layout (#STKPWHRAO*EUFRPBLGTSDZ) for now. More complex and adaptable stroke types may be implemented in the future.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct EnglishStroke([u8; 3]);

impl EnglishStroke {
    pub fn as_bytes(&self) -> &[u8; 3] {
        &self.0
    }

    pub fn into_bytes(self) -> [u8; 3] {
        self.0
    }
}

impl From<u32> for EnglishStroke {
    /// Converts from a u32 where the stroke data is left-aligned within the last 24 bits (i.e. one trailing zero bit)
    fn from(input: u32) -> Self {
        debug_assert_eq!(input & 1, 0);
        Self([
            (input >> 16 & 0b11111111) as u8,
            (input >> 8 & 0b11111111) as u8,
            (input >> 0 & 0b11111111) as u8,
        ])
    }
}

impl Display for EnglishStroke {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let contains_vowel = (self.0[1] & 0b11110000) > 0;

        for (byte_index, byte) in self.0.iter().enumerate() {
            for bit_index in 0..8 {
                let mask = 1 << (7 - bit_index);
                let index = byte_index * 8 + bit_index;

                // Write a `-` after the last vowel bit if there is no vowel
                if !contains_vowel && index == 13 {
                    f.write_char('-')?;
                }

                // Write the corresponding human-readable key
                if (byte & mask) > 0 {
                    f.write_char(KEYMAP[index])?;
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod does {
    use super::*;
    use alloc::string::ToString;

    #[test]
    fn convert_from_u32_correctly() {
        //                 #STKPWHR AO*EU FRPBLGTSDZ
        let input: u32 = 0b10000010_10001_0010000001_0;
        let stroke = EnglishStroke::from(input);
        assert_eq!(stroke.to_string(), "#HAUPZ");
    }

    #[test]
    fn output_correct_characters() {
        let stroke = EnglishStroke([u8::MAX, u8::MAX, 0b11111110]);
        assert_eq!(stroke.to_string(), "#STKPWHRAO*EUFRPBLGTSDZ");
    }

    #[test]
    fn use_hyphen() {
        let stroke = EnglishStroke([0b00000010, 0b00000_001, 0b0000000_0]);
        assert_eq!(stroke.to_string(), "H-P");
    }

    #[test]
    fn omit_hyphen() {
        let stroke = EnglishStroke([0b01100000, 0b01000_001, 0b0000000_0]);
        assert_eq!(stroke.to_string(), "STOP");
    }
}
