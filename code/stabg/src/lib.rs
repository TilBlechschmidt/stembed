#![no_std]
#![feature(doc_auto_cfg, doc_cfg)]
#![cfg_attr(feature = "nightly", feature(generic_associated_types))]
#![allow(dead_code)]
#![feature(async_closure, unboxed_closures)]

#[cfg(feature = "alloc")]
extern crate alloc;

mod context;
mod executor;
mod identifier;
mod macros;
mod queue;
mod registry;
mod stack;

pub mod processor;

pub use context::*;
pub use executor::Executor;
pub use identifier::*;
pub use queue::*;
pub use stack::*;

/// Automatically implements the [`Identifiable`](self::Identifiable) trait.
///
/// Nothing really special, but makes the code a little more legible!
/// You can optionally add a version attribute which will concatenate the provided
/// version string separated by a `-`.
///
/// # Examples
///
/// ```
/// use stabg::{Identifiable, Identifier};
///
/// #[derive(Identifiable)]
/// #[identifiable(name = "test.type-1")]
/// struct TestType1(u8);
///
/// #[derive(Identifiable)]
/// #[identifiable(name = "test.type", version = "2")]
/// struct TestType2(u8);
///
///# fn main() {
/// assert_eq!(TestType1::IDENTIFIER, "test.type-1");
/// assert_eq!(TestType2::IDENTIFIER, "test.type-2");
///# }
/// ```
#[cfg(feature = "derive")]
pub use stabg_derive::Identifiable;
