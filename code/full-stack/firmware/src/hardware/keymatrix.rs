use core::future::{ready, Future};
use core::ops::Add;
use embassy_executor::time::{Duration, Timer};
use embassy_nrf::gpio::{AnyPin, Input, Level, Output, OutputDrive, Pull};
use embassy_util::Either::*;
use embassy_util::{select, select_all};
use engine::{input::KeyPosition, InputState};
use futures::{stream, Stream, StreamExt};

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
}

impl<M: ScannableMatrix + Unpin> MatrixScanner<M> {
    pub fn new(matrix: M, active_scan_interval: Duration) -> Self {
        Self {
            matrix,
            active_scan_interval,
        }
    }

    pub fn into_state_stream(self) -> impl Stream<Item = InputState> {
        let mut previous_data: Option<InputState> = None;

        struct ScanState<M> {
            sleeping: bool,
            scan_interval: Duration,
            matrix: M,
        }

        let initial_state = ScanState {
            sleeping: false,
            scan_interval: self.active_scan_interval,
            matrix: self.matrix,
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
        .filter(move |data| {
            let is_included = Some(*data) != previous_data;
            previous_data = Some(*data);
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
