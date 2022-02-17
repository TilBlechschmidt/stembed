use super::SerialPort;
use crate::{
    core::Key::{self, *},
    input::{InputKeyState, InputSource},
};

const START_BYTE_MASK: u8 = 0b10000000;
const BYTE_COUNT: usize = 6;
const BITS_PER_BYTE: usize = 7;

pub struct GeminiPR(SerialPort);

impl GeminiPR {
    pub fn new(port: SerialPort) -> Self {
        Self(port)
    }
}

impl InputSource<42> for GeminiPR {
    #[rustfmt::skip]
    const FRIENDLY_NAMES: [&'static str; 42] = [
        "Fn", "#1", "#2", "#3", "#4", "#5", "#6",
        "S1-", "S2-", "T-", "K-", "P-", "W-", "H-",
        "R-", "A-", "O-", "*1", "*2", "res1", "res2",
        "pwr", "*3", "*4", "-E", "-U", "-F", "-R",
        "-P", "-B", "-L", "-G", "-T", "-S", "-D",
        "#7", "#8", "#9", "#A", "#B", "#C", "-Z"
    ];

    #[rustfmt::skip]
    const DEFAULT_KEYMAP: [Key; 42] = [
        Extra("FN1"), Left('#'), Left('#'), Left('#'), Left('#'), Left('#'), Left('#'),
        Left('S'), Left('S'), Left('T'), Left('K'), Left('P'), Left('W'), Left('H'),
        Left('R'), Middle('A'), Middle('O'), Middle('*'), Middle('*'), Extra("FN2"), Extra("FN3"),
        Extra("FN4"), Middle('*'), Middle('*'), Middle('E'), Middle('U'), Right('F'), Right('R'),
        Right('P'), Right('B'), Right('L'), Right('G'), Right('T'), Right('S'), Right('D'),
        Left('#'), Left('#'), Left('#'), Left('#'), Left('#'), Left('#'), Right('Z'),
    ];

    type Error = std::io::Error;

    fn scan(&mut self) -> Result<[InputKeyState; 42], Self::Error> {
        let mut buffer = [0u8; BYTE_COUNT];
        let mut index = 0;

        while index < 6 {
            let byte = self.0.read_u8()?;

            if byte & START_BYTE_MASK > 0 {
                buffer[index] = byte;
                index = 1;
            } else if index > 0 {
                buffer[index] = byte;
                index += 1;
            }
        }

        let mut key_states = [false; 42];
        for byte_index in 0..BYTE_COUNT {
            for bit_index in 0..BITS_PER_BYTE {
                let pressed = buffer[byte_index] & (1 << (6 - bit_index)) > 0;
                let index = byte_index * BITS_PER_BYTE + bit_index as usize;
                key_states[index] = pressed;
            }
        }

        Ok(key_states)
    }
}
