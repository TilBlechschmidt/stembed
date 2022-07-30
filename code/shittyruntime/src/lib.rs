#![cfg_attr(not(feature = "std"), no_std)]
#![feature(type_alias_impl_trait)]
#![feature(generic_associated_types)]

pub mod cofit;
pub mod firmware;
pub mod input;

mod runtime;
pub use runtime::messaging;
pub use runtime::Runtime;
