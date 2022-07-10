#![no_std]
#![no_main]
#![macro_use]
#![feature(generic_associated_types)]
#![feature(type_alias_impl_trait)]
#![feature(array_try_map)]

use defmt_rtt as _; // global logger
use panic_probe as _; // global panic handler

defmt::timestamp!("{=u64}", embassy::time::Instant::now().as_millis());

mod hardware;
mod logic;

use embassy_nrf::gpio::Pin;
use hardware::usb;

const FLASH_SIZE: usize = 2usize.pow(16) * 256;

const KEYMAP: &[&[u8]] = &[
    &[41, 12],         // #
    &[37, 36],         // S-
    &[40],             // T-
    &[39],             // K-
    &[43],             // P-
    &[42],             // W-
    &[46],             // H-
    &[45],             // R-
    &[44],             // A
    &[47],             // O
    &[49, 48, 17, 16], // *
    &[15],             // E
    &[9],              // U
    &[11],             // -F
    &[10],             // -R
    &[14],             // -P
    &[13],             // -B
    &[8],              // -L
    &[7],              // -G
    &[5],              // -T
    &[4],              // -S
    &[2],              // -D
    &[1],              // -Z
];

// TODO There is the possiblity to set the keyboard language tag which gives the OS a hint on which keyboard layout to use!
// TODO Change asserts in hardware modules to returning a result so we can fail gracefully

// TODO By giving ownership of all the 'static stuff to the `Runtime`, we can keep borrows around and send the runtime into the task thus removing all the `static` madness.

#[embassy::main]
async fn main(s: embassy::executor::Spawner, p: embassy_nrf::Peripherals) {
    defmt::info!("Hello world!");
    keyboard_shenanigans(s, p).await;
    // usb_shenanigans(s, p).await;
    // flash_shenanigans(s, p).await;
}

async fn keyboard_shenanigans(_s: embassy::executor::Spawner, p: embassy_nrf::Peripherals) {
    use embassy::time::Duration;
    use futures::{pin_mut, StreamExt};
    use hardware::keymatrix::*;
    use logic::*;
    use shittyengine::Stroke;

    const ACTIVE_SCAN_PERIOD: Duration = Duration::from_millis(15);
    const REPEAT_INTERVAL: Duration = Duration::from_millis(150);
    const REPEAT_MAX_TAP_DIST: Duration = Duration::from_millis(500);

    let rows_left = [p.P0_24.degrade(), p.P0_22.degrade(), p.P0_05.degrade()];
    let rows_right = [p.P0_28.degrade(), p.P0_03.degrade(), p.P1_11.degrade()];

    let columns_left = [
        p.P0_07.degrade(),
        p.P0_08.degrade(),
        p.P1_09.degrade(),
        p.P1_06.degrade(),
        p.P0_12.degrade(),
        p.P0_04.degrade(),
    ];
    let columns_right = [
        p.P1_13.degrade(),
        p.P0_10.degrade(),
        p.P0_09.degrade(),
        p.P0_29.degrade(),
        p.P0_02.degrade(),
        p.P0_30.degrade(),
    ];

    let matrix_left = KeyMatrix::new(rows_left, columns_left);
    let matrix_right = KeyMatrix::new(rows_right, columns_right);
    let mut scanner = MatrixScanner::new(matrix_left + matrix_right, ACTIVE_SCAN_PERIOD);

    let mut grouper = KeypressGrouper::new(GroupingMode::FirstUp);
    let repeater = KeypressRepeater::new(REPEAT_INTERVAL, REPEAT_MAX_TAP_DIST);

    let state_stream = scanner
        .state()
        .map(|state| map_keys(state, KEYMAP))
        .map(|keys: u32| {
            let mut states = [false; 23];
            for i in 0..23 {
                states[i] = (keys & (0b1 << i)) > 0;
            }
            states
        });

    pin_mut!(state_stream);

    let grouped_stream = repeater
        .apply_grouped_repeat(&mut state_stream, &mut grouper)
        .map(|keys: [bool; 23]| {
            let mut state = 0u32;
            for i in 0..23 {
                if keys[i] {
                    state |= 0b1 << i;
                }
            }
            state
        })
        .map(Stroke::from_right_aligned);

    pin_mut!(grouped_stream);

    while let Some(stroke) = grouped_stream.next().await {
        defmt::info!("Stroke: {:?}", stroke);
    }
}

async fn flash_shenanigans(_s: embassy::executor::Spawner, p: embassy_nrf::Peripherals) {
    hardware::uicr::ensure_nfc_disabled();
    hardware::power::enable_voltage_regulator(p.P1_00);

    defmt::info!("Initializing flash ...");

    let flash = hardware::flash::configure::<FLASH_SIZE>(
        p.QSPI,
        p.P0_26.degrade(),
        p.P0_06.degrade(),
        p.P0_13.degrade(),
        p.P0_15.degrade(),
        p.P0_17.degrade(),
        p.P0_20.degrade(),
    )
    .await;
}

async fn usb_shenanigans(s: embassy::executor::Spawner, p: embassy_nrf::Peripherals) {
    // Create configs for all peripherals
    let config_usb = embassy_usb::Config::new(0xc0de, 0xcafe);

    // Build the peripheral runtimes
    defmt::info!("Initializing USB peripheral");
    let mut runtime_usb = usb::configure(p.USBD, config_usb);

    defmt::info!("Adding HID USB endpoints");
    let (keyboard, runtime_keyboard) = usb::keyboard::configure(&mut runtime_usb);
    let (sender, receiver, runtime_channel) = usb::channel::configure(&mut runtime_usb);

    // Spawn the runtime tasks
    defmt::info!("Spawning runtimes");
    s.must_spawn(usb::run(runtime_usb));
    s.must_spawn(usb::keyboard::run(runtime_keyboard));
    s.must_spawn(usb::channel::run(runtime_channel));

    loop {
        let command = receiver.recv().await;
        // defmt::info!("Received command {:?}", command);

        match command.identifier {
            // Simple echo command
            13 => sender.send(command).await,

            // Write w/ keyboard command
            42 => {
                let length = command.payload[0] as usize;
                if let Ok(text) = core::str::from_utf8(&command.payload[1..1 + length]) {
                    keyboard.send_str(&text).await;
                } else {
                    defmt::warn!("Command contained invalid UTF-8 data");
                }
            }

            // Fallback
            _ => defmt::warn!("Received unknown command {}", command.identifier),
        }
    }
}
