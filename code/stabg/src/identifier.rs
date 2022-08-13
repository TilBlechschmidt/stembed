/// Internal identifier linked to a globally unique [`Identifier`](Identifier) through a [`Registry`](super::Registry)
pub type ShortID = u8;

/// Globally unique identifier string in reverse-domain notation
///
/// When creating new processors and adding types for them, you usually have to create one of these.
/// To prevent collisions with other plugins, you should name them based on a domain you own or are sure
/// of that nobody else will use for their plugin. It is customary to then reverse this domain name and append
/// the name of the type you are working with.
///
/// Examples:
/// - `dev.blechschmidt.formattingStyle`
/// - `com.example.command.reset`
///
/// **The prefix `core` is reserved for internal use of the hosting application.**
///
/// # Versioning
///
/// Your types may change over time. When you are on your own, this does not matter. However, as soon as
/// other plugins rely on your types, you should properly version them! The convention is to append a [SemVer](https://semver.org)
/// string like `0.2.10` to your type, separated by a hyphen.
///
/// Examples:
/// - `dev.blechschmidt.formattingStyle-0.0.1`
/// - `com.example.command.reset-1.9.13`
///
/// If you write your plugin in Rust, you can use the [`Identifiable`](stabg_derive::Identifiable) macro to do this automatically for you.
pub type Identifier = &'static str;

/// Requires the implementing type to carry an [`Identifier`](Identifier)
pub trait Identifiable {
    const IDENTIFIER: Identifier;
}
