#![no_std]
#![feature(generic_associated_types)]
#![feature(type_alias_impl_trait)]

#[cfg(feature = "alloc")]
extern crate alloc;

const ORTHOGRAPHIC_SUFFIX_LENGTH: usize = 5;

const NODE_HEADER_SIZE: usize = 1 + 1 + 3;
/// Maximum length of the child prefix array stored in the radix tree node
const PREFIX_ARRAY_SIZE_LIMIT: usize = 256;
/// Maximum number of bytes a translation may use in serialized form
const TRANSLATION_SIZE_LIMIT: usize = 256;

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
