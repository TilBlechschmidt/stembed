use core::ops::Add;
use embassy::util::Either::*;
use embassy::util::{select, select_all};
use embassy_nrf::gpio::{AnyPin, Input, Level, Output, OutputDrive, Pull};

pub struct KeyMatrix<'p, const ROWS: usize, const COLUMNS: usize> {
    rows: [Input<'p, AnyPin>; ROWS],
    columns: [Output<'p, AnyPin>; COLUMNS],
}

impl<'p, const ROWS: usize, const COLUMNS: usize> KeyMatrix<'p, ROWS, COLUMNS> {
    pub fn new(rows: [AnyPin; ROWS], columns: [AnyPin; COLUMNS]) -> Self {
        assert!(
            rows.len() * columns.len() <= 32,
            "there may not be more than 32 keys"
        );

        let rows = rows.map(|pin| Input::new(pin, Pull::Down));
        let columns = columns.map(|pin| Output::new(pin, Level::Low, OutputDrive::HighDrive));

        Self { rows, columns }
    }

    pub fn scan_once(&mut self) -> u32 {
        let mut state = 0u32;

        for column in self.columns.iter_mut() {
            column.set_high();

            for row in self.rows.iter_mut() {
                let key_state = if row.is_low() { 0 } else { 1 };
                state = state << 1;
                state = state | key_state;
            }

            column.set_low();
        }

        state
    }

    /// Waits until a key is pressed. Returns whether the key was already pressed and no waiting was performed.
    pub async fn wait_for_press(&mut self) -> bool {
        for column in self.columns.iter_mut() {
            column.set_high();
        }

        let mut reset = || {
            for column in self.columns.iter_mut() {
                column.set_low();
            }
        };

        for row in self.rows.iter_mut() {
            if row.is_high() {
                reset();
                return true;
            }
        }

        let row_interrupts: [_; ROWS] = defmt::unwrap!(array_from_iter(
            self.rows.iter_mut().map(|row| row.wait_for_high())
        ));

        select_all(row_interrupts).await;
        reset();

        false
    }
}

pub struct JoinedKeyMatrix<
    'p,
    const ROWS_LEFT: usize,
    const ROWS_RIGHT: usize,
    const COLUMNS_LEFT: usize,
    const COLUMNS_RIGHT: usize,
> {
    left: KeyMatrix<'p, ROWS_LEFT, COLUMNS_LEFT>,
    right: KeyMatrix<'p, ROWS_RIGHT, COLUMNS_RIGHT>,
}

impl<
        'p,
        const ROWS_LEFT: usize,
        const ROWS_RIGHT: usize,
        const COLUMNS_LEFT: usize,
        const COLUMNS_RIGHT: usize,
    > JoinedKeyMatrix<'p, ROWS_LEFT, ROWS_RIGHT, COLUMNS_LEFT, COLUMNS_RIGHT>
{
    pub fn scan_once(&mut self) -> u64 {
        ((self.left.scan_once() as u64) << 32) + self.right.scan_once() as u64
    }

    pub async fn wait_for_press(&mut self) -> bool {
        match select(self.left.wait_for_press(), self.right.wait_for_press()).await {
            First(immediate) => immediate,
            Second(immediate) => immediate,
        }
    }
}

impl<
        'p,
        const ROWS_LEFT: usize,
        const ROWS_RIGHT: usize,
        const COLUMNS_LEFT: usize,
        const COLUMNS_RIGHT: usize,
    > Add<KeyMatrix<'p, ROWS_RIGHT, COLUMNS_RIGHT>> for KeyMatrix<'p, ROWS_LEFT, COLUMNS_LEFT>
{
    type Output = JoinedKeyMatrix<'p, ROWS_LEFT, ROWS_RIGHT, COLUMNS_LEFT, COLUMNS_RIGHT>;

    fn add(self, rhs: KeyMatrix<'p, ROWS_RIGHT, COLUMNS_RIGHT>) -> Self::Output {
        JoinedKeyMatrix {
            left: self,
            right: rhs,
        }
    }
}

fn array_from_iter<I: IntoIterator, const N: usize>(iter: I) -> Option<[I::Item; N]> {
    let mut iter = iter.into_iter();
    let out = [(); N].try_map(|()| iter.next())?;

    if iter.next().is_some() {
        None
    } else {
        Some(out)
    }
}
