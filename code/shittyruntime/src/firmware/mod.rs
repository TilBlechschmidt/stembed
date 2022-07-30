//! Traits for abstracting away firmware details

// Internally, this module also contains abstractions for async executors.
// These cover both the traits in the `sync` & `time` submodules, but also
// implementations of said traits in the `executor_support` module which
// chooses between implementations based on feature flags and type aliases.

pub mod executor_support;

mod flash;
mod peripherals;
mod sync;
mod time;

pub(crate) use flash::*;
pub use peripherals::*;
pub use sync::*;
pub use time::*;
