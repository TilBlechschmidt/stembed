#![no_std]
#![feature(doc_auto_cfg)]
#![cfg_attr(feature = "embedded", feature(generic_associated_types))]
#![allow(dead_code)]

#[cfg(feature = "alloc")]
extern crate alloc;

#[cfg(feature = "alloc")]
mod collection;
mod context;
mod executor;
mod identifier;
mod processor;
mod registry;
mod stack;

#[cfg(feature = "alloc")]
pub mod desktop {
    pub use super::collection::*;
    pub use super::processor::{InitializationContext, Processor, TypeUsage};
}

pub mod embedded {
    pub use super::processor::EmbeddedProcessor;
    pub use super::registry::FixedSizeRegistry;
    pub use super::stack::FixedSizeStack;
}

/// Collection of error types returned by this crate
///
/// Most of them are either related to out-of-memory conditions or caused by the
/// user-provided processor.
pub mod error {
    pub use super::context::ExecutionContextError;
    pub use super::processor::ExecutionError;
    #[cfg(feature = "alloc")]
    pub use super::processor::InitializationError;
    pub use super::registry::RegistryError;
    pub use super::stack::StackError;
}

pub use context::{ExecutionBranch, ExecutionContext};
pub use executor::Executor;
pub use identifier::*;
pub use registry::Registry;
pub use stack::Stack;

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
