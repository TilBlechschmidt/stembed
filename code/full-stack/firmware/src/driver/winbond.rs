use core::future::Future;
use embassy_nrf::{
    gpio::{Output, Pin},
    spim::{Instance, Spim},
};
use embassy_util::yield_now;
use embedded_storage::nor_flash::{ErrorType, NorFlashError, NorFlashErrorKind};
use embedded_storage_async::nor_flash::{AsyncNorFlash, AsyncReadNorFlash};

const JEDEC_W25Q128FV: [u8; 3] = [239, 64, 24];
const FLASH_SIZE: u32 = 4096 * 4096;
const PAGE_SIZE: u32 = 256;
const ERASE_SIZE: u32 = 4096;

#[allow(dead_code)]
enum Command {
    DisableWrite = 0x04,
    EnableWrite = 0x06,
    EnableVolatileStatusRegisterWrite = 0x50,

    ReadUniqueIdentifier = 0x4B,
    ReadJedecIdentifier = 0x9F,
    ReadStatusRegister1 = 0x05,
    ReadStatusRegister2 = 0x35,
    ReadStatusRegister3 = 0x15,
    ReadData = 0x03,

    PageProgram = 0x02,
    EraseSector = 0x20,
    EraseChip = 0xC7,

    PowerDown = 0xB9,
    PowerUp = 0xAB,
}
use Command::*;

#[derive(Debug)]
pub enum Error {}

pub struct WinbondFlash<'s, 'cs, S: Instance, CS: Pin> {
    spi: Spim<'s, S>,
    chip_select: Output<'cs, CS>,
}

impl<'s, 'cs, S: Instance, CS: Pin> WinbondFlash<'s, 'cs, S, CS> {
    pub fn new(spi: Spim<'s, S>, chip_select: Output<'cs, CS>) -> Self {
        Self { spi, chip_select }
    }

    async fn cmd(&mut self, command: Command) {
        self.chip_select.set_low();

        // Since all failure modes are "API misuse" we should just panic to inform the dev
        self.spi
            .write(&[command as u8])
            .await
            .expect("SPI write failed");

        self.chip_select.set_high();
    }

    async fn cmd_data(&mut self, command: Command, data: &[u8]) {
        self.chip_select.set_low();

        self.spi
            .write(&[command as u8])
            .await
            .expect("SPI write failed");

        self.spi.write(data).await.expect("SPI write failed");

        self.chip_select.set_high();
    }

    async fn cmd_data_return(&mut self, command: Command, data: &[u8], response: &mut [u8]) {
        self.chip_select.set_low();

        self.spi
            .write(&[command as u8])
            .await
            .expect("SPI write failed");

        self.spi.write(data).await.expect("SPI write failed");
        self.spi.read(response).await.expect("SPI read failed");

        self.chip_select.set_high();
    }

    async fn cmd_return(&mut self, command: Command, response: &mut [u8]) {
        self.chip_select.set_low();

        self.spi
            .write(&[command as u8])
            .await
            .expect("SPI write failed");

        self.spi.read(response).await.expect("SPI read failed");

        self.chip_select.set_high();
    }

    pub async fn sleep(&mut self) {
        self.cmd(PowerDown).await;
    }

    pub async fn wake(&mut self) {
        self.cmd(PowerUp).await;
    }

    async fn is_idle(&mut self) -> bool {
        let mut status_register = [0; 1];
        self.cmd_return(ReadStatusRegister1, &mut status_register)
            .await;

        status_register[0] & 0b1 == 0
    }

    async fn idle(&mut self) {
        // TODO There is a way of continously reading the status register
        //      Maybe we can somehow use that with EasyDMA (doubt it)?
        while !self.is_idle().await {
            yield_now().await;
        }
    }

    /// Fetches the manufacturer and product identifier
    async fn jedec_id(&mut self) -> [u8; 3] {
        let mut jedec = [0; 3];
        self.cmd_return(ReadJedecIdentifier, &mut jedec).await;
        jedec
    }

    /// Fetches the factory-set unique identifier
    pub async fn unique_id(&mut self) -> u64 {
        let mut buf = [0; 12];
        self.cmd_return(ReadUniqueIdentifier, &mut buf).await;

        // Discard the leading dummy bytes
        let mut id = [0; 8];
        id.copy_from_slice(&buf[4..]);

        u64::from_be_bytes(id)
    }

    /// Reads an arbitrary amount of data
    pub async fn read(&mut self, address: u32, data: &mut [u8]) {
        assert!(address + (data.len() as u32) < FLASH_SIZE);
        let address_bytes = address.to_be_bytes();

        self.cmd_data_return(ReadData, &address_bytes[1..], data)
            .await;
    }

    /// Writes data into a 256-byte aligned page and wraps around at the page boundary if there are more bytes than remaining page
    async fn write_page(&mut self, address: u32, data: &[u8]) {
        assert!(address + (data.len() as u32) < FLASH_SIZE);

        let address_bytes = address.to_be_bytes();

        self.cmd(EnableWrite).await;

        self.chip_select.set_low();

        self.spi
            .write(&[PageProgram as u8])
            .await
            .expect("SPI write failed");

        self.spi
            .write(&address_bytes[1..])
            .await
            .expect("SPI write failed");

        self.spi.write(data).await.expect("SPI write failed");

        self.chip_select.set_high();

        self.idle().await;
    }

    /// Writes an arbitrary amount of data at any address, across flash pages
    pub async fn write(&mut self, address: u32, data: &[u8]) {
        let page_remaining_bytes = PAGE_SIZE - (address % PAGE_SIZE);

        if data.len() as u32 > page_remaining_bytes {
            self.write_page(address, &data[0..page_remaining_bytes as usize])
                .await;

            for (i, chunk) in data[page_remaining_bytes as usize..]
                .chunks(PAGE_SIZE as usize)
                .enumerate()
            {
                self.write_page(address + page_remaining_bytes + i as u32 * PAGE_SIZE, chunk)
                    .await;
            }
        } else {
            self.write_page(address, data).await;
        }
    }

    /// Erases a 4096 byte sector at the given address. Panics if the address is not aligned to 4k.
    /// Do note that a given sector can only be erased a limited number of times, so be careful!
    pub async fn erase_sector(&mut self, address: u32) {
        assert!(address + ERASE_SIZE < FLASH_SIZE);
        assert_eq!(address % ERASE_SIZE, 0);
        let address_bytes = address.to_be_bytes();

        self.cmd(EnableWrite).await;
        self.cmd_data(EraseSector, &address_bytes[1..]).await;
        self.idle().await;
    }

    /// Erases all sectors of the chip at once
    pub async fn erase_chip(&mut self) {
        self.cmd(EnableWrite).await;
        self.cmd(EraseChip).await;
        self.idle().await;
    }

    pub async fn init(&mut self) {
        // Power-cycle the device to make sure its awake
        self.sleep().await;
        self.wake().await;

        // Verify its identifier
        let jedec = self.jedec_id().await;
        assert_eq!(jedec, JEDEC_W25Q128FV);
    }
}

impl<'s, 'cs, S: Instance, CS: Pin> ErrorType for WinbondFlash<'s, 'cs, S, CS> {
    type Error = Error;
}

impl NorFlashError for Error {
    fn kind(&self) -> NorFlashErrorKind {
        NorFlashErrorKind::Other
    }
}

impl<'s, 'cs, S: Instance, CS: Pin> AsyncReadNorFlash for WinbondFlash<'s, 'cs, S, CS> {
    const READ_SIZE: usize = 1;

    type ReadFuture<'a> = impl Future<Output = Result<(), Error>> + 'a
    where
        Self: 'a;

    fn read<'a>(&'a mut self, offset: u32, bytes: &'a mut [u8]) -> Self::ReadFuture<'a> {
        // TODO Convert out-of-range panics to errors
        async move {
            self.read(offset, bytes).await;
            Ok(())
        }
    }

    fn capacity(&self) -> usize {
        FLASH_SIZE as usize
    }
}

impl<'s, 'cs, S: Instance, CS: Pin> AsyncNorFlash for WinbondFlash<'s, 'cs, S, CS> {
    const WRITE_SIZE: usize = 1;
    const ERASE_SIZE: usize = ERASE_SIZE as usize;

    type EraseFuture<'a> = impl Future<Output = Result<(), Error>> + 'a
    where
        Self: 'a;

    type WriteFuture<'a> = impl Future<Output = Result<(), Error>> + 'a
    where
        Self: 'a;

    fn erase<'a>(&'a mut self, from: u32, to: u32) -> Self::EraseFuture<'a> {
        async move {
            for address in (from..to).step_by(Self::ERASE_SIZE) {
                self.erase_sector(address).await;
            }
            Ok(())
        }
    }

    fn write<'a>(&'a mut self, offset: u32, bytes: &'a [u8]) -> Self::WriteFuture<'a> {
        async move {
            self.write(offset, bytes).await;
            Ok(())
        }
    }
}
