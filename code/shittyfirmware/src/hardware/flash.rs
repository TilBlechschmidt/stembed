use defmt::{debug, info, unwrap};
use embassy_nrf::{
    gpio::AnyPin,
    interrupt,
    peripherals::QSPI,
    qspi::{self, Config, Qspi},
};
use embedded_storage_async::nor_flash::AsyncNorFlash;

const PAGE_SIZE: usize = 4096;

#[derive(defmt::Format)]
#[repr(C, align(4))]
struct AlignedBuf([u8; PAGE_SIZE]);

/// Configures a W25Q128FV flash chip connected to the given pins for operation
pub async fn configure<const FLASH_SIZE: usize>(
    peripheral: QSPI,
    clock: AnyPin,
    chip_select: AnyPin,
    io0: AnyPin,
    io1: AnyPin,
    io2: AnyPin,
    io3: AnyPin,
) -> impl AsyncNorFlash {
    let irq = interrupt::take!(QSPI);
    let mut config = Config::default();
    config.read_opcode = qspi::ReadOpcode::READ4O;
    config.write_opcode = qspi::WriteOpcode::PP4O;
    config.write_page_size = qspi::WritePageSize::_256BYTES;
    config.deep_power_down = None;

    let mut qspi = Qspi::<_, FLASH_SIZE>::new(
        peripheral,
        irq,
        clock,
        chip_select,
        io0,
        io1,
        io2,
        io3,
        config,
    );

    // Send the chip to power down
    unwrap!(qspi.custom_instruction(0xB9, &[], &mut []).await);

    // Power it back up again
    unwrap!(qspi.custom_instruction(0xAB, &[], &mut []).await);

    // Enable writes to the status registers
    unwrap!(qspi.custom_instruction(0x06, &[], &mut []).await);

    // Activate QSPI mode
    unwrap!(qspi.custom_instruction(0x31, &[0x02], &mut []).await);

    // Verify that QSPI mode was enabled
    let mut status = [0u8; 1];
    unwrap!(qspi.custom_instruction(0x35, &[], &mut status).await);
    debug!("flash chip status register #2: {=[u8]:x}", status);
    assert_eq!(status[0] & 0b10, 2);

    // Read the chip ID to verify we are talking to the correct chip (Winbond, W25Q128FV)
    let mut id = [0u8; 3];
    unwrap!(qspi.custom_instruction(0x9F, &[], &mut id).await);
    debug!("flash chip JEDEC identifier: {=[u8]:x}", id);
    assert_eq!(id, [0xEF, 0x40, 0x18]);

    qspi
}

/// Confirms that the flash chip operates correctly by writing eight pages of data and reading them back
async fn verify_flash_operation<const FLASH_SIZE: usize>(qspi: &mut Qspi<'_, QSPI, FLASH_SIZE>) {
    let mut buf = AlignedBuf([1u8; PAGE_SIZE]);

    let pattern = |a: u32| (a ^ (a >> 8) ^ (a >> 16) ^ (a >> 24)) as u8;

    for i in 0..8 {
        info!("page {:?}: erasing... ", i);
        unwrap!(qspi.erase(i * PAGE_SIZE).await);

        for j in 0..PAGE_SIZE {
            buf.0[j] = pattern((j + i * PAGE_SIZE) as u32);
        }

        info!("programming...");
        unwrap!(qspi.write(i * PAGE_SIZE, &buf.0).await);
    }

    let mut different_bytes = 0;
    let mut success = true;
    for i in 0..8 {
        info!("page {:?}: reading... ", i);
        unwrap!(qspi.read(i * PAGE_SIZE, &mut buf.0).await);
        // debug!("read: {=[u8]:x}", buf.0);

        let previous = different_bytes;
        info!("verifying...");
        for j in 0..PAGE_SIZE {
            if buf.0[j] != pattern((j + i * PAGE_SIZE) as u32) {
                success = false;
                different_bytes += 1;
                defmt::warn!("invalid byte at offset {}", j);
            }
        }

        if previous != different_bytes {
            defmt::warn!("failed to verify page");
        }

        cortex_m::asm::delay(1000000);
    }

    info!(
        "done! success = {}, different_bytes = {}, last = {}",
        success, different_bytes, buf.0[0]
    );
}
