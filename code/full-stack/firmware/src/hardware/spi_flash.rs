use embassy_nrf::{
    gpio::{AnyPin, Level, Output, OutputDrive},
    interrupt,
    peripherals::SPI2,
    spim::{Config, Frequency, Spim},
};
use embedded_storage_async::nor_flash::AsyncNorFlash;

use crate::driver::WinbondFlash;

pub async fn configure(
    peripheral: SPI2,
    clock: AnyPin,
    chip_select: AnyPin,
    miso: AnyPin,
    mosi: AnyPin,
) -> impl AsyncNorFlash {
    let mut config = Config::default();
    config.frequency = Frequency::M32;

    let irq = interrupt::take!(SPIM2_SPIS2_SPI2);
    let spim = Spim::new(peripheral, irq, clock, miso, mosi, config);
    let chip_select = Output::new(chip_select, Level::High, OutputDrive::Standard);

    let mut flash = WinbondFlash::new(spim, chip_select);
    flash.init().await;

    flash
}
