use core::future::{ready, Future};
use core::ops::Add;
use embassy::time::{Duration, Timer};
use embassy::util::Either::*;
use embassy::util::{select, select_all};
use embassy_nrf::gpio::{AnyPin, Input, Level, Output, OutputDrive, Pull};
use futures::{stream, Stream, StreamExt};

pub trait ScannableMatrix {
    type Output: Eq;
    type WaitFuture<'a>: Future<Output = ()> + 'a
    where
        Self: 'a;

    const NO_KEYPRESS_VALUE: Self::Output;

    fn scan_once(&mut self) -> Self::Output;
    fn wait_for_press<'a>(&'a mut self) -> Self::WaitFuture<'a>;
}

pub struct MatrixScanner<M: ScannableMatrix + Unpin> {
    matrix: M,
    active_scan_interval: Duration,

    previous_data: Option<M::Output>,
}

impl<M: ScannableMatrix + Unpin> MatrixScanner<M>
where
    M::Output: Copy,
{
    pub fn new(matrix: M, active_scan_interval: Duration) -> Self {
        Self {
            matrix,
            active_scan_interval,
            previous_data: None,
        }
    }

    pub fn state<'m>(&'m mut self) -> impl Stream<Item = M::Output> + 'm {
        self.previous_data = None;

        struct ScanState<'m, M> {
            sleeping: bool,
            scan_interval: Duration,
            matrix: &'m mut M,
        }

        let initial_state = ScanState {
            sleeping: false,
            scan_interval: self.active_scan_interval,
            matrix: &mut self.matrix,
        };

        stream::unfold(initial_state, |mut state| async move {
            if state.sleeping {
                state.matrix.wait_for_press().await;
                state.sleeping = false;
            } else {
                Timer::after(state.scan_interval).await;
            }

            let data = state.matrix.scan_once();

            if data == M::NO_KEYPRESS_VALUE {
                state.sleeping = true;
            }

            Some((data, state))
        })
        .filter(|data| {
            let is_included = Some(*data) != self.previous_data;
            self.previous_data = Some(*data);
            ready(is_included)
        })
    }
}

pub struct KeyMatrix<'p, const ROWS: usize, const COLUMNS: usize> {
    rows: [Input<'p, AnyPin>; ROWS],
    columns: [Output<'p, AnyPin>; COLUMNS],
}

impl<'p, const ROWS: usize, const COLUMNS: usize> KeyMatrix<'p, ROWS, COLUMNS> {
    pub fn new(rows: [AnyPin; ROWS], columns: [AnyPin; COLUMNS]) -> Self {
        assert!(
            rows.len() * columns.len() <= u32::BITS as usize,
            "there may not be more than 32 keys"
        );

        let rows = rows.map(|pin| Input::new(pin, Pull::Down));
        let columns = columns.map(|pin| Output::new(pin, Level::Low, OutputDrive::HighDrive));

        Self { rows, columns }
    }
}

impl<'p, const ROWS: usize, const COLUMNS: usize> ScannableMatrix for KeyMatrix<'p, ROWS, COLUMNS> {
    type Output = u32;
    type WaitFuture<'a> = impl Future<Output = ()> + 'a
    where
        Self: 'a;

    const NO_KEYPRESS_VALUE: Self::Output = 0;

    fn scan_once(&mut self) -> u32 {
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

    fn wait_for_press<'a>(&'a mut self) -> Self::WaitFuture<'a> {
        async move {
            for column in self.columns.iter_mut() {
                column.set_high();
            }

            let row_interrupts: [_; ROWS] = defmt::unwrap!(array_from_iter(
                self.rows.iter_mut().map(|row| row.wait_for_high())
            ));

            select_all(row_interrupts).await;

            for column in self.columns.iter_mut() {
                column.set_low();
            }
        }
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
    > ScannableMatrix for JoinedKeyMatrix<'p, ROWS_LEFT, ROWS_RIGHT, COLUMNS_LEFT, COLUMNS_RIGHT>
{
    type Output = u64;
    type WaitFuture<'a> = impl Future<Output = ()> + 'a
    where
        Self: 'a;

    const NO_KEYPRESS_VALUE: Self::Output = 0;

    fn scan_once(&mut self) -> u64 {
        ((self.left.scan_once() as u64) << 32) + self.right.scan_once() as u64
    }

    fn wait_for_press<'a>(&'a mut self) -> Self::WaitFuture<'a> {
        async move {
            match select(self.left.wait_for_press(), self.right.wait_for_press()).await {
                First(immediate) => immediate,
                Second(immediate) => immediate,
            }
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

pub fn map_keys(input: u64, keymap: &[&[u8]]) -> u32 {
    let mut output = 0u32;

    for (target_index, input_indices) in keymap.iter().rev().enumerate() {
        for input_index in input_indices.iter() {
            let value = (input >> input_index) & 1;
            output |= (value as u32) << target_index;
        }
    }

    output
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
