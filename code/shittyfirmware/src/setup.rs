use crate::hardware::{
    self,
    usb::{self, keyboard::Keyboard},
};

use super::hardware::keymatrix::*;
use embassy::time::Duration;
use embassy_nrf::{
    gpio::{AnyPin, Pin},
    peripherals,
};
use embedded_storage_async::nor_flash::AsyncNorFlash;
use futures::{Sink, Stream};
use shittyengine::Stroke;
use shittyruntime::{cofit::Transport, firmware::AsyncOutputCommand, input::*};

macro_rules! make_strokemap {
    ( $([ $($position_str:expr),* ]),* ) => {
        &[
            $(
                &[$( KeyPosition::from($position_str), )*],
            )*
        ]
    };
}

const USB_VID: u16 = 0xc0de;
const USB_PID: u16 = 0xcafe;

const FLASH_SIZE: usize = 2usize.pow(16) * 256;

const ACTIVE_SCAN_PERIOD: Duration = Duration::from_millis(15);
const REPEAT_INTERVAL: Duration = Duration::from_millis(75);
const REPEAT_TRIGGER_DELAY: Duration = Duration::from_millis(150);
const REPEAT_MAX_TAP_DIST: Duration = Duration::from_millis(250);

#[rustfmt::skip]
const KEYMAP_LEFT: &[Option<KeyPosition>] = make_keymap![
    "---", "LP1", "LR1", "LM1", "LI1", "LET1",
    "---", "LP2", "LR2", "LM2", "LI2", "LET2",
    "---", "---", "---", "LM3", "LI3", "LET3"
];

#[rustfmt::skip]
const KEYMAP_RIGHT: &[Option<KeyPosition>] = make_keymap![
    "REL1", "RI1", "RM1", "RR1", "RP1", "RET1",
    "REL2", "RI2", "RM2", "RR2", "RP2", "RET2",
    "REL3", "RI3", "RM3", "---", "---", "---"
];

const STROKE_MAP: &[&[Option<KeyPosition>]] = make_strokemap![
    ["LM3", "RM3"],                   // #
    ["LP1", "LP2"],                   // S-
    ["LR1"],                          // T-
    ["LR2"],                          // K-
    ["LM1"],                          // P-
    ["LM2"],                          // W-
    ["LI1"],                          // H-
    ["LI2"],                          // R-
    ["LI3"],                          // A
    ["LET3"],                         // O
    ["LET1", "LET2", "REL1", "REL2"], // *
    ["REL3"],                         // E
    ["RI3"],                          // U
    ["RI1"],                          // -F
    ["RI2"],                          // -R
    ["RM1"],                          // -P
    ["RM2"],                          // -B
    ["RR1"],                          // -L
    ["RR2"],                          // -G
    ["RP1"],                          // -T
    ["RP2"],                          // -S
    ["RET1"],                         // -D
    ["RET2"]                          // -Z
];

fn stroke_from_input(input: InputState) -> Stroke {
    let mut state = 0u32;

    for (keys, i) in STROKE_MAP.iter().zip((0..STROKE_MAP.len()).rev()) {
        for key in keys.iter() {
            if Some(true) == key.map(|k| input.is_set(k)) {
                state |= 1 << i;
            }
        }
    }

    Stroke::from_right_aligned(state)
}

async fn setup_flash(
    voltage_regulator_enable: peripherals::P1_00,
    qspi: peripherals::QSPI,
    clock: peripherals::P0_26,
    chip_select: peripherals::P0_06,
    io0: peripherals::P0_13,
    io1: peripherals::P0_15,
    io2: peripherals::P0_17,
    io3: peripherals::P0_20,
) -> impl AsyncNorFlash {
    hardware::uicr::ensure_nfc_disabled();
    hardware::power::enable_voltage_regulator(voltage_regulator_enable);

    defmt::info!("Initializing flash");

    hardware::flash::configure::<FLASH_SIZE>(
        qspi,
        clock.degrade(),
        chip_select.degrade(),
        io0.degrade(),
        io1.degrade(),
        io2.degrade(),
        io3.degrade(),
    )
    .await
}

fn setup_usb(
    s: &embassy::executor::Spawner,
    usbd: peripherals::USBD,
) -> (Keyboard<'static>, impl Transport<64>) {
    // Create configs for all peripherals
    let mut config = embassy_usb::Config::new(USB_VID, USB_PID);
    config.manufacturer = Some("Evil Steno Corp");
    config.product = Some("Goldcrest v0.0.1");
    config.serial_number = Some("0.0.1");

    config.max_power = 100;
    config.max_packet_size_0 = 64;
    config.supports_remote_wakeup = true;

    // Required for windows compatiblity.
    // https://developer.nordicsemi.com/nRF_Connect_SDK/doc/1.9.1/kconfig/CONFIG_CDC_ACM_IAD.html#help
    config.device_class = 0xEF;
    config.device_sub_class = 0x02;
    config.device_protocol = 0x01;
    config.composite_with_iads = true;

    // Build the peripheral runtimes
    defmt::info!("Initializing USB peripheral");
    let mut runtime_usb = usb::configure(usbd, config);

    defmt::info!("Adding HID USB endpoints");
    let (keyboard, runtime_keyboard) = usb::keyboard::configure(&mut runtime_usb);
    let (channel, runtime_channel) = usb::channel::configure(&mut runtime_usb);

    // Spawn the runtime tasks
    defmt::info!("Spawning runtimes");
    s.must_spawn(usb::run(runtime_usb));
    s.must_spawn(usb::keyboard::run(runtime_keyboard));
    s.must_spawn(usb::channel::run(runtime_channel));

    (keyboard, channel)
}

fn setup_input(
    rows_left: [AnyPin; 3],
    rows_right: [AnyPin; 3],
    columns_left: [AnyPin; 6],
    columns_right: [AnyPin; 6],
) -> impl Stream<Item = InputState> {
    defmt::info!("Configuring keymatrix");
    let matrix_left = KeyMatrix::new(rows_left, columns_left, KEYMAP_LEFT);
    let matrix_right = KeyMatrix::new(rows_right, columns_right, KEYMAP_RIGHT);
    let scanner = MatrixScanner::new(matrix_left + matrix_right, ACTIVE_SCAN_PERIOD);

    // let mut grouper = KeypressGrouper::new(GroupingMode::FirstUp);
    // let repeater = KeypressRepeater::new(
    //     REPEAT_INTERVAL,
    //     REPEAT_MAX_TAP_DIST,
    //     REPEAT_TRIGGER_DELAY,
    //     EmbassyTimer,
    // );

    // let state_stream = scanner.state();

    // pin_mut!(state_stream);

    // let grouped_stream = repeater
    //     .apply_grouped_repeat(&mut state_stream, &mut grouper)
    //     .map(stroke_from_input);

    // pin_mut!(grouped_stream);

    // while let Some(stroke) = grouped_stream.next().await {
    //     defmt::info!("Stroke: {:?}", stroke);
    // }

    scanner.into_state_stream()
}

pub async fn peripherals(
    s: &embassy::executor::Spawner,
    p: embassy_nrf::Peripherals,
) -> shittyruntime::firmware::Peripherals<
    impl Stream<Item = InputState>,
    impl Transport<64>,
    impl AsyncNorFlash,
    impl Sink<AsyncOutputCommand>,
> {
    let rows_left = [p.P0_24.degrade(), p.P0_22.degrade(), p.P0_05.degrade()];
    let rows_right = [p.P0_28.degrade(), p.P0_03.degrade(), p.P1_11.degrade()];

    let columns_left = [
        p.P0_04.degrade(),
        p.P0_12.degrade(),
        p.P1_06.degrade(),
        p.P1_09.degrade(),
        p.P0_08.degrade(),
        p.P0_07.degrade(),
    ];
    let columns_right = [
        p.P1_13.degrade(),
        p.P0_09.degrade(),
        p.P0_10.degrade(),
        p.P0_29.degrade(),
        p.P0_02.degrade(),
        p.P0_30.degrade(),
    ];

    let input = setup_input(rows_left, rows_right, columns_left, columns_right);
    let (keyboard, usb_channel) = setup_usb(s, p.USBD);
    let flash = setup_flash(
        p.P1_00, p.QSPI, p.P0_26, p.P0_06, p.P0_13, p.P0_15, p.P0_17, p.P0_20,
    )
    .await;

    shittyruntime::firmware::Peripherals {
        input,
        usb_output: keyboard.into_sink(),
        usb_channel,
        flash,
    }
}
