#![no_std]
#![cfg_attr(feature = "embedded", feature(generic_associated_types))]
#![allow(dead_code)]

#[cfg(feature = "alloc")]
extern crate alloc;

mod executor;
mod identifier;

#[cfg(feature = "alloc")]
mod collection;

pub use executor::Executor;
pub use identifier::{Identifiable, Identifier, ShortID};

#[cfg(feature = "alloc")]
pub use collection::ProcessorCollection;

pub mod context;
pub mod processor;
pub mod registry;
pub mod stack;
