//! Recursive-descent parser for JSON dictionaries

mod dict;
mod stroke;

pub use dict::{dict, CommandList, Outline};
use stroke::stroke;
