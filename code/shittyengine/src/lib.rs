// #![no_std]

#[cfg(feature = "alloc")]
extern crate alloc;

const ORTHOGRAPHIC_SUFFIX_LENGTH: usize = 5;

const NODE_HEADER_SIZE: usize = 1 + 1 + 3;
const PREFIX_ARRAY_SIZE_LIMIT: usize = 256;
const TRANSLATION_SIZE_LIMIT: usize = PREFIX_ARRAY_SIZE_LIMIT;

pub mod dict;
pub mod formatter;
pub mod matcher;
pub mod output;

#[cfg(feature = "compile")]
pub mod compile;

mod buffer;
use buffer::*;

mod stroke;
pub use stroke::EnglishStroke as Stroke;
