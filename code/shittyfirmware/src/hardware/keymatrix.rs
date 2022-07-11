use core::future::{ready, Future};
use core::ops::Add;
use embassy::time::{Duration, Timer};
use embassy::util::Either::*;
use embassy::util::{select, select_all};
use embassy_nrf::gpio::{AnyPin, Input, Level, Output, OutputDrive, Pull};
use futures::{stream, Stream, StreamExt};
use shittyruntime::input::{InputState, KeyPosition};

pub trait ScannableMatrix {
    type WaitFuture<'a>: Future<Output = ()> + 'a
    where
        Self: 'a;

    fn scan_once(&mut self) -> InputState;
    fn wait_for_press<'a>(&'a mut self) -> Self::WaitFuture<'a>;
}

pub struct MatrixScanner<M: ScannableMatrix + Unpin> {
    matrix: M,
    active_scan_interval: Duration,

    previous_data: Option<InputState>,
}

impl<M: ScannableMatrix + Unpin> MatrixScanner<M> {
    pub fn new(matrix: M, active_scan_interval: Duration) -> Self {
        Self {
            matrix,
            active_scan_interval,
            previous_data: None,
        }
    }

    pub fn state<'m>(&'m mut self) -> impl Stream<Item = InputState> + 'm {
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
            state.sleeping = data.is_empty();

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
    keymap: &'p [Option<KeyPosition>],
}

impl<'p, const ROWS: usize, const COLUMNS: usize> KeyMatrix<'p, ROWS, COLUMNS> {
    pub fn new(
        rows: [AnyPin; ROWS],
        columns: [AnyPin; COLUMNS],
        keymap: &'p [Option<KeyPosition>],
    ) -> Self {
        assert_eq!(
            ROWS * COLUMNS,
            keymap.len(),
            "keymap does not contain a mapping for each key"
        );

        let rows = rows.map(|pin| Input::new(pin, Pull::Down));
        let columns = columns.map(|pin| Output::new(pin, Level::Low, OutputDrive::HighDrive));

        Self {
            rows,
            columns,
            keymap,
        }
    }
}

impl<'p, const ROWS: usize, const COLUMNS: usize> ScannableMatrix for KeyMatrix<'p, ROWS, COLUMNS> {
    type WaitFuture<'a> = impl Future<Output = ()> + 'a
    where
        Self: 'a;

    fn scan_once(&mut self) -> InputState {
        let mut state = InputState::EMPTY;

        for (ci, column) in self.columns.iter_mut().enumerate() {
            column.set_high();

            for (ri, row) in self.rows.iter_mut().enumerate() {
                let i = ri * COLUMNS + ci;

                if let Some(position) = self.keymap[i] {
                    if row.is_high() {
                        state.set(position);
                    }
                }
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
    type WaitFuture<'a> = impl Future<Output = ()> + 'a
    where
        Self: 'a;

    fn scan_once(&mut self) -> InputState {
        self.left.scan_once() + self.right.scan_once()
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

fn array_from_iter<I: IntoIterator, const N: usize>(iter: I) -> Option<[I::Item; N]> {
    let mut iter = iter.into_iter();
    let out = [(); N].try_map(|()| iter.next())?;

    if iter.next().is_some() {
        None
    } else {
        Some(out)
    }
}
