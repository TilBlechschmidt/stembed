use core::ops::{Add, AddAssign, Sub};

use super::{
    KeyColumn::*,
    KeyPosition::{self, *},
    KeyRow::*,
};

const BIT_ORDER: &[KeyPosition] = &[
    Left(Pinky, Above),
    Left(Pinky, Top),
    Left(Pinky, Bottom),
    Left(Pinky, Below),
    Left(Ring, Above),
    Left(Ring, Top),
    Left(Ring, Bottom),
    Left(Ring, Below),
    Left(Middle, Above),
    Left(Middle, Top),
    Left(Middle, Bottom),
    Left(Middle, Below),
    Left(Index, Above),
    Left(Index, Top),
    Left(Index, Bottom),
    Left(Index, Below),
    Left(Thumb, Above),
    Left(Thumb, Top),
    Left(Thumb, Bottom),
    Left(Thumb, Below),
    Left(ExtraLeading, Above),
    Left(ExtraLeading, Top),
    Left(ExtraLeading, Bottom),
    Left(ExtraLeading, Below),
    Left(ExtraTrailing, Above),
    Left(ExtraTrailing, Top),
    Left(ExtraTrailing, Bottom),
    Left(ExtraTrailing, Below),
    Right(Pinky, Above),
    Right(Pinky, Top),
    Right(Pinky, Bottom),
    Right(Pinky, Below),
    Right(Ring, Above),
    Right(Ring, Top),
    Right(Ring, Bottom),
    Right(Ring, Below),
    Right(Middle, Above),
    Right(Middle, Top),
    Right(Middle, Bottom),
    Right(Middle, Below),
    Right(Index, Above),
    Right(Index, Top),
    Right(Index, Bottom),
    Right(Index, Below),
    Right(Thumb, Above),
    Right(Thumb, Top),
    Right(Thumb, Bottom),
    Right(Thumb, Below),
    Right(ExtraLeading, Above),
    Right(ExtraLeading, Top),
    Right(ExtraLeading, Bottom),
    Right(ExtraLeading, Below),
    Right(ExtraTrailing, Above),
    Right(ExtraTrailing, Top),
    Right(ExtraTrailing, Bottom),
    Right(ExtraTrailing, Below),
];

#[derive(PartialEq, Eq)]
pub enum KeyEvent {
    Up,
    Down,
}

/// Intermediate representation of keyboard input state
// Internal representation in the last 28-bits of the u32
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct InputState(u64);

impl InputState {
    pub const EMPTY: Self = Self(0);

    pub const fn new() -> Self {
        Self::EMPTY
    }

    pub fn set(&mut self, key: KeyPosition) -> Self {
        self.0 |= 1 << Self::bit_index(key);
        InputState(self.0)
    }

    pub fn unset(&mut self, key: KeyPosition) -> Self {
        self.0 &= !(1 << Self::bit_index(key));
        InputState(self.0)
    }

    pub fn is_set(&self, key: KeyPosition) -> bool {
        (self.0 & (1 << Self::bit_index(key))) > 0
    }

    pub fn is_empty(&self) -> bool {
        self.0 == 0
    }

    pub(super) fn edges_toward(&self, next: Self) -> impl Iterator<Item = (KeyPosition, KeyEvent)> {
        self.into_iter()
            .zip(next.into_iter())
            .filter_map(|(prev, next)| {
                debug_assert_eq!(prev.0, next.0);
                match (prev.1, next.1) {
                    (false, true) => Some((prev.0, KeyEvent::Down)),
                    (true, false) => Some((prev.0, KeyEvent::Up)),
                    (false, false) | (true, true) => None,
                }
            })
    }

    fn bit_index(key: KeyPosition) -> usize {
        BIT_ORDER
            .iter()
            .enumerate()
            .find_map(|(i, x)| if *x == key { Some(i) } else { None })
            .expect("IR bit order incomplete")
    }
}

impl Add for InputState {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl Sub for InputState {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0 & !rhs.0)
    }
}

impl AddAssign for InputState {
    fn add_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0
    }
}

impl IntoIterator for InputState {
    type Item = (KeyPosition, bool);
    type IntoIter = impl Iterator<Item = Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        (0..BIT_ORDER.len()).map(move |i| (BIT_ORDER[i], self.is_set(BIT_ORDER[i])))
    }
}
