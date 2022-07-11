use core::{
    fmt::{Debug, Display, Write},
    ops::{Add, AddAssign},
};

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
    pub fn pressed_key_count(&self) -> u32 {
        self.0[0].count_ones() + self.0[1].count_ones() + self.0[2].count_ones()
    }

    pub fn is_empty(&self) -> bool {
        self.0[0] == 0 && self.0[1] == 0 && self.0[2] == 0
    }

    pub fn as_bytes(&self) -> &[u8; 3] {
        &self.0
    }

    pub fn into_bytes(self) -> [u8; 3] {
        self.0
    }

    fn contains_vowel(&self) -> bool {
        (self.0[1] & 0b11111000) > 0
    }

    // Converts from a u32 where the stroke data is right-aligned and covers the last 23 bits
    pub fn from_right_aligned(input: u32) -> Self {
        Self::from(input << 1)
    }
}

#[cfg(feature = "defmt")]
impl defmt::Format for EnglishStroke {
    fn format(&self, f: defmt::Formatter) {
        let contains_vowel = self.contains_vowel();

        for (byte_index, byte) in self.0.iter().enumerate() {
            for bit_index in 0..8 {
                let mask = 1 << (7 - bit_index);
                let index = byte_index * 8 + bit_index;

                // Write a `-` after the last vowel bit if there is no vowel
                if !contains_vowel && index == 13 {
                    defmt::write!(f, "-");
                }

                // Write the corresponding human-readable key
                if (byte & mask) > 0 {
                    defmt::write!(f, "{}", KEYMAP[index]);
                }
            }
        }
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

impl Add for EnglishStroke {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        let mut bytes = self.into_bytes();
        let rhs_bytes = rhs.into_bytes();
        bytes[0] |= rhs_bytes[0];
        bytes[1] |= rhs_bytes[1];
        bytes[2] |= rhs_bytes[2];
        Self(bytes)
    }
}

impl AddAssign for EnglishStroke {
    fn add_assign(&mut self, rhs: Self) {
        let rhs_bytes = rhs.into_bytes();
        self.0[0] |= rhs_bytes[0];
        self.0[1] |= rhs_bytes[1];
        self.0[2] |= rhs_bytes[2];
    }
}

impl Display for EnglishStroke {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let contains_vowel = self.contains_vowel();

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

impl Debug for EnglishStroke {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        (self as &dyn Display).fmt(f)
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
    fn output_vowels_without_dash() {
        //                   #STKPWHR AO*EU FRPBLGTSDZ
        let input_a: u32 = 0b00000000_10000_0000000000_0;
        let input_o: u32 = 0b00000000_01000_0000000000_0;
        let input_e: u32 = 0b00000000_00010_0000000000_0;
        let input_u: u32 = 0b00000000_00001_0000000000_0;
        let data = [
            (input_a, "A"),
            (input_o, "O"),
            (input_e, "E"),
            (input_u, "U"),
        ];

        for (input, output) in data {
            let stroke = EnglishStroke::from(input);
            assert_eq!(stroke.to_string(), output);
        }
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
