#![cfg_attr(not(feature = "std"), no_std)]
#![feature(doc_auto_cfg)]
#![feature(doc_cfg)]
#![feature(type_alias_impl_trait)]
#![feature(generic_associated_types)]

mod message;

#[cfg(feature = "api")]
pub mod api;

#[cfg(feature = "runtime")]
mod runtime;
#[cfg(feature = "runtime")]
pub use runtime::*;
