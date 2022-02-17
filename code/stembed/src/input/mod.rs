use crate::core::Key;

#[cfg(feature = "serial")]
pub mod serial;

pub type InputKeyState = bool;

pub trait InputSource<const KEY_COUNT: usize> {
    const KEY_COUNT: usize = KEY_COUNT;
    const FRIENDLY_NAMES: [&'static str; KEY_COUNT];
    const DEFAULT_KEYMAP: [Key; KEY_COUNT];

    type Error;

    fn scan(&mut self) -> Result<[InputKeyState; KEY_COUNT], Self::Error>;
}
