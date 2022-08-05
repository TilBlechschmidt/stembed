pub mod flash;
pub mod keymatrix;
pub mod usb;

pub mod spi_flash;

pub mod clock {
    pub fn enable_high_frequency_oscillator() {
        let clock: embassy_nrf::pac::CLOCK = unsafe { core::mem::transmute(()) };
        clock.tasks_hfclkstart.write(|w| unsafe { w.bits(1) });
        while clock.events_hfclkstarted.read().bits() != 1 {}
    }
}

pub mod power {
    use embassy_nrf::{gpio::Output, peripherals::P1_00};
    use embassy_util::Forever;

    static VOLTAGE_REGULATOR_ENABLE: Forever<Output<P1_00>> = Forever::new();

    /// Activates the external 3.3V voltage regulator present on the BlueMacro840 board, used to power peripherals like the flash.
    /// For more details, look at the board schematic: http://nrf52.jpconstantineau.com/docs/bluemacro840_v1#schematic
    pub fn enable_voltage_regulator(pin: embassy_nrf::peripherals::P1_00) {
        let mut voltage_regulator_enable = embassy_nrf::gpio::Output::new(
            pin,
            embassy_nrf::gpio::Level::High,
            embassy_nrf::gpio::OutputDrive::HighDrive,
        );
        voltage_regulator_enable.set_high();
        VOLTAGE_REGULATOR_ENABLE.put(voltage_regulator_enable);
    }
}

pub mod uicr {
    /// Verifies that pins P0.09 and P0.10 are configured as GPIOs and not as NFC antenna pins.
    /// This setting is persisted in the UICR across resets and will only be written if required.
    ///
    /// See nRF documentation for more details:
    ///
    /// https://infocenter.nordicsemi.com/index.jsp?topic=%2Fnrf52.v1.7%2FChunk832797900.html&anchor=register.NFCPINS
    ///
    /// https://infocenter.nordicsemi.com/pdf/nRF52840_PS_v1.1.pdf (page 578)
    pub fn ensure_nfc_disabled() {
        let uicr: embassy_nrf::pac::UICR = unsafe { core::mem::transmute(()) };
        let nvmc: embassy_nrf::pac::NVMC = unsafe { core::mem::transmute(()) };

        if uicr.nfcpins.read().protect().is_nfc() {
            defmt::debug!("NFC enabled, disabling ...");

            nvmc.config.write(|w| w.wen().wen());
            uicr.nfcpins.write(|w| w.protect().disabled());
            nvmc.config.reset();
            // A reset is probably required, but is dangerous without safe-guards.
            // If the write was unsuccessful, we could end up in a reboot-loop which might repeatedly write to flash.
            // cortex_m::peripheral::SCB::sys_reset();
        }
    }
}
