#![no_std]
#![no_main]
use core::sync::atomic::{AtomicUsize, Ordering};
use cortex_m_rt::entry;
use nrf52840_hal::gpio::*;

pub use defmt::*;
use defmt_rtt as _; // global logger
use panic_probe as _;

use embedded_hal::blocking::spi::Transfer;

#[entry]
fn main() -> ! {
    let p = nrf52840_hal::pac::Peripherals::take().unwrap();
    let port0 = p0::Parts::new(p.P0);

    // let cs = p.P0_13;
    // let sck = p.P0_15;
    // let mosi = p.P0_17;
    // let miso = p.P0_20;

    let cs = port0.p0_13.into_push_pull_output(Level::High);
    let spiclk = port0.p0_15.into_push_pull_output(Level::Low).degrade();
    let spimosi = port0.p0_17.into_push_pull_output(Level::Low).degrade();
    let spimiso = port0.p0_22.into_floating_input().degrade();

    let pins = nrf52840_hal::spi::Pins {
        sck: spiclk,
        miso: Some(spimiso),
        mosi: Some(spimosi),
    };

    let mut spi = nrf52840_hal::spi::Spi::new(p.SPI1, pins, nrf52840_hal::spi::Frequency::M1, nrf52840_hal::spi::MODE_0);
    let mut sd = embedded_sdmmc::SdMmcSpi::new(spi, cs);

    let result = sd.acquire();
    defmt::warn!("SD Init result: {:?}", result.is_err());
    let card = result.unwrap();

    // loop {
    //     let mut tx = [1, 2, 3, 4];
    //     let res = spi.transfer(&mut tx).unwrap();
    //     println!("received: {}", res);
    //     cortex_m::asm::delay(1000000);
    // }

    loop {
        match card.card_size_bytes() {
            Ok(size) => defmt::info!("SD card size: {:?}", size),
            Err(err) => defmt::warn!("SD card size failed.")
        }
        cortex_m::asm::delay(1000000);
    }
}

defmt::timestamp! {"{=u64}", {
        static COUNT: AtomicUsize = AtomicUsize::new(0);
        COUNT.fetch_add(1, Ordering::Relaxed) as u64
    }
}
