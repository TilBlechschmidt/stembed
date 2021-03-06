// #![cfg_attr(not(feature = "std"), no_std)]
#![feature(generic_associated_types)]
#![feature(type_alias_impl_trait)]

#[macro_use]
extern crate alloc;
mod constants;

pub mod core;
pub mod input;
pub mod io;
pub mod output;
pub mod serialize;

#[cfg(feature = "compile")]
pub mod compile;

#[cfg(feature = "import")]
pub mod import;
