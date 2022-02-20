use embassy_nrf::gpio::{Output, AnyPin, Input};
use stembed::{input::{InputSource, InputKeyState}, core::Key};
use embedded_hal::digital::v2::InputPin;

pub struct KeymatrixInput<'d> {
    pub columns_left: [Output<'d, AnyPin>; 6],
    pub rows_left: [Input<'d, AnyPin>; 3],
    pub columns_right: [Output<'d, AnyPin>; 6],
    pub rows_right: [Input<'d, AnyPin>; 3],
}

impl<'d> KeymatrixInput<'d> {
    fn scan_matrix(&mut self, left: bool) -> u32 {
        let columns = if left {
            &mut self.columns_left
        } else {
            &mut self.columns_right
        };
        let rows = if left {
            &mut self.rows_left
        } else {
            &mut self.rows_right
        };

        // Rework this with interrupts so we can sleep while no keys are pressed
        // (keep all columns high, wait for any row to become high, then take everything low and do a scan)
        let mut state: u32 = 0;

        for column in columns.iter_mut() {
            column.set_high();

            for row in rows.iter_mut() {
                let key_state = if row.is_low().unwrap() { 0 } else { 1 };
                state = state << 1;
                state = state | key_state;
            }

            column.set_low();
        }

        state
    }

    fn scan_stroke(&mut self) -> [bool; 30] {
        let mut combined_state_left = 0;
        let mut combined_state_right = 0;

        // Since we have "dead keys" in the matrix, there is a need to remap to get rid of those
        // Additionally, we can use this opportunity to reorder some keys (for dev experience really)
        //
        // Left:
        // **O HRA PW# TK- SS- ---
        // 111 000 000 000 000 000
        //
        // Right:
        // **E FRU PB# LG- TS- DZ-
        // 111 000 000 000 000 000
        //
        // Output:
        // **O HRA PW# TK- SS | **E FRU PB# LG- TS- DZ
        let state_mappings = [
            // Left side
            (true, 0b000_000_000_000_000_100), // FN1
            (true, 0b000_000_000_000_000_010), // FN2
            (true, 0b000_000_001_000_000_000), // #
            (true, 0b000_000_000_000_100_000), // S
            (true, 0b000_000_000_000_010_000), // S
            (true, 0b000_000_000_100_000_000), // T
            (true, 0b000_000_000_010_000_000), // K
            (true, 0b000_000_100_000_000_000), // P
            (true, 0b000_000_010_000_000_000), // W
            (true, 0b000_100_000_000_000_000), // H
            (true, 0b000_010_000_000_000_000), // R
            (true, 0b000_001_000_000_000_000), // A
            (true, 0b001_000_000_000_000_000), // O
            (true, 0b100_000_000_000_000_000), // *1
            (true, 0b010_000_000_000_000_000), // *2
            // Right side
            (false, 0b100_000_000_000_000_000), // *3
            (false, 0b010_000_000_000_000_000), // *4
            (false, 0b001_000_000_000_000_000), // E
            (false, 0b000_001_000_000_000_000), // U
            (false, 0b000_100_000_000_000_000), // F
            (false, 0b000_010_000_000_000_000), // R
            (false, 0b000_000_100_000_000_000), // P
            (false, 0b000_000_010_000_000_000), // B
            (false, 0b000_000_000_100_000_000), // L
            (false, 0b000_000_000_010_000_000), // G
            (false, 0b000_000_000_000_100_000), // T
            (false, 0b000_000_000_000_010_000), // S
            (false, 0b000_000_000_000_000_100), // D
            (false, 0b000_000_000_000_000_010), // Z
            (false, 0b000_000_001_000_000_000), // #
        ];

        loop {
            let state_left = self.scan_matrix(true);
            let state_right = self.scan_matrix(false);

            if state_left == 0
                && state_right == 0
                && (combined_state_left > 0 || combined_state_right > 0)
            {
                return state_mappings.map(|(left, mask)| {
                    let state = if left {
                        combined_state_left
                    } else {
                        combined_state_right
                    };

                    state & mask > 0
                });
            } else {
                combined_state_left |= state_left;
                combined_state_right |= state_right;
            }
        }
    }
}

impl<'d> InputSource<30> for KeymatrixInput<'d> {
    #[rustfmt::skip]
    const FRIENDLY_NAMES: [&'static str; 30] = [
        "FN1", "FN2", "#L", "S1-", "S2-", "T-", "K-", "P-", "W-", "H-", "R-", "A", "O", "*1", "*2",
        "*3", "*4", "E", "U", "-F", "-R", "-P", "-B", "-L", "-G", "-T", "-S", "-D", "-Z", "#R"
    ];

    #[rustfmt::skip]
    const DEFAULT_KEYMAP: [Key; 30] = [
        Key::Extra("FN1"), Key::Extra("FN2"), Key::Left('#'), Key::Left('S'), Key::Left('S'), Key::Left('T'), Key::Left('K'), Key::Left('P'), Key::Left('W'), Key::Left('H'), Key::Left('R'), Key::Middle('A'), Key::Middle('O'), Key::Middle('*'), Key::Middle('*'),
        Key::Middle('*'), Key::Middle('*'), Key::Middle('E'), Key::Middle('U'), Key::Right('F'), Key::Right('R'), Key::Right('P'), Key::Right('B'), Key::Right('L'), Key::Right('G'), Key::Right('T'), Key::Right('S'), Key::Right('D'), Key::Right('Z'), Key::Left('#')
    ];

    type Error = ();

    fn scan(&mut self) -> Result<[InputKeyState; 30], Self::Error> {
        Ok(self.scan_stroke())
    }
}
