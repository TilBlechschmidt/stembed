use super::hardware::keymatrix::*;
use crate::hardware::{
    self,
    usb::{self, keyboard::Keyboard},
};
use cofit::Transport;
use embassy_executor::time::Duration;
use embassy_nrf::{
    gpio::{AnyPin, Pin},
    interrupt, peripherals,
    usb::PowerUsb,
};
use embedded_storage_async::nor_flash::AsyncNorFlash;
use engine::{input::KeyPosition, InputState, OutputCommand};
use futures::{Sink, Stream};

#[macro_export]
macro_rules! make_keymap {
    ( $($position_str:expr),* ) => {
        &[
            $( KeyPosition::from($position_str), )*
        ]
    };
}

const USB_VID: u16 = 0xc0de;
const USB_PID: u16 = 0xcafe;

const FLASH_SIZE: usize = 2usize.pow(16) * 256;

const ACTIVE_SCAN_PERIOD: Duration = Duration::from_millis(15);

// #[rustfmt::skip]
// const KEYMAP_LEFT: &[Option<KeyPosition>] = make_keymap![
//     "---", "LP1", "LR1", "LM1", "LI1", "LET1",
//     "---", "LP2", "LR2", "LM2", "LI2", "LET2",
//     "---", "---", "---", "LM3", "LI3", "LET3"
// ];

// #[rustfmt::skip]
// const KEYMAP_RIGHT: &[Option<KeyPosition>] = make_keymap![
//     "REL1", "RI1", "RM1", "RR1", "RP1", "RET1",
//     "REL2", "RI2", "RM2", "RR2", "RP2", "RET2",
//     "REL3", "RI3", "RM3", "---", "---", "---"
// ];

#[rustfmt::skip]
const KEYMAP: &[Option<KeyPosition>] = make_keymap![
     "---", "LP1", "LR1", "LM1", "LI1", "LET1", "REL1", "RI1", "RM1", "RR1", "RP1", "RET1",
     "---", "LP2", "LR2", "LM2", "LI2", "LET2", "REL2", "RI2", "RM2", "RR2", "RP2", "RET2",
     "---", "---", "---", "LM3", "LI3", "LET3", "REL3", "RI3", "RM3", "---", "---", "---"
];

async fn setup_flash(
    qspi: peripherals::QSPI,
    clock: peripherals::P0_26,
    chip_select: peripherals::P0_06,
    io0: peripherals::P0_13,
    io1: peripherals::P0_15,
    io2: peripherals::P0_17,
    io3: peripherals::P0_20,
) -> impl AsyncNorFlash {
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

async fn setup_spi_flash(
    spi: peripherals::SPI2,
    clock: peripherals::P0_06,
    chip_select: peripherals::P0_05,
    miso: peripherals::P0_20,
    mosi: peripherals::P0_26,
) -> impl AsyncNorFlash {
    defmt::info!("Initializing flash");

    hardware::spi_flash::configure(
        spi,
        clock.degrade(),
        chip_select.degrade(),
        miso.degrade(),
        mosi.degrade(),
    )
    .await
}

fn setup_usb(
    s: &embassy_executor::executor::Spawner,
    usbd: peripherals::USBD,
) -> (Keyboard<'static>, impl Transport<63>) {
    // Create config for the USB peripheral
    let mut config = embassy_usb::Config::new(USB_VID, USB_PID);
    config.manufacturer = Some("Evil Steno Corp");
    config.product = Some("Goldcrest v0.0.1");
    config.serial_number = Some("0.0.1");

    config.max_power = 500;
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
    // TODO This interrupt can not be used with BLE, switch to a SignalledSupply
    //      For details, see https://github.com/embassy-rs/embassy/commit/8785fbc6f1a1227115d3ffa6a6c19035bed6ef8c
    let power_irq = interrupt::take!(POWER_CLOCK);
    let mut runtime_usb = usb::configure(usbd, config, PowerUsb::new(power_irq));

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
    rows: [AnyPin; 3],
    columns: [AnyPin; 12],
) -> impl Stream<Item = InputState> {
    defmt::info!("Configuring keymatrix");

    let matrix = KeyMatrix::new(rows, columns, KEYMAP);
    let scanner = MatrixScanner::new(matrix, ACTIVE_SCAN_PERIOD);

    scanner.into_state_stream()
}

pub async fn peripherals(
    s: &embassy_executor::executor::Spawner,
    p: embassy_nrf::Peripherals,
) -> runtime::HardwareStack<
    impl Stream<Item = InputState>,
    impl Transport<63>,
    impl AsyncNorFlash,
    impl Sink<OutputCommand>,
> {
    hardware::uicr::ensure_nfc_disabled();
    hardware::power::enable_voltage_regulator(p.P1_00);

    let rows = [p.P0_28.degrade(), p.P0_03.degrade(), p.P1_11.degrade()];
    let columns = [
        p.P0_04.degrade(),
        p.P0_12.degrade(),
        p.P1_06.degrade(),
        p.P1_09.degrade(),
        p.P0_08.degrade(),
        p.P0_07.degrade(),
        p.P1_13.degrade(),
        p.P0_09.degrade(),
        p.P0_10.degrade(),
        p.P0_29.degrade(),
        p.P0_02.degrade(),
        p.P0_30.degrade(),
    ];

    let input = setup_input(rows, columns);
    let (keyboard, usb_channel) = setup_usb(s, p.USBD);
    // let flash = setup_flash(
    //     p.QSPI, p.P0_26, p.P0_06, p.P0_13, p.P0_15, p.P0_17, p.P0_20,
    // )
    // .await;

    let flash = setup_spi_flash(p.SPI2, p.P0_06, p.P0_05, p.P0_20, p.P0_26).await;

    runtime::HardwareStack {
        input,
        usb_output: keyboard.into_sink(),
        usb_channel,
        flash,
    }
}
