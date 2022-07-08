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

use embassy_nrf::gpio::Pin;
use hardware::usb;

const FLASH_SIZE: usize = 2usize.pow(16) * 256;

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
    const ACTIVE_SCAN_PERIOD: embassy::time::Duration = embassy::time::Duration::from_millis(50);

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

    let matrix_left = hardware::keymatrix::KeyMatrix::new(rows_left, columns_left);
    let matrix_right = hardware::keymatrix::KeyMatrix::new(rows_right, columns_right);
    let mut matrix = matrix_left + matrix_right;

    // TODO Move most of this into matrix and have it return a stream (or at least a function `next_state` that can be called repeatedly)
    loop {
        let immediate = matrix.wait_for_press().await;
        let state = matrix.scan_once();

        // TODO We never receive the `0` state when releasing the keys

        defmt::info!(
            "Matrix state: {=u64:b} {}",
            state,
            if immediate { "" } else { "zzZ" }
        );

        if immediate {
            embassy::time::Timer::after(ACTIVE_SCAN_PERIOD).await;
        }
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
