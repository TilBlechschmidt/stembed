use defmt::{trace, warn};
use embassy::time::{Duration, Timer};
use embassy_nrf::{gpio::Output, spim::Spim};

#[derive(defmt::Format)]
pub enum SDError {
    SPIError(embassy_nrf::spim::Error),
    TimeoutWaitNoBusy,
    TimeoutCommand(u8),
    CardNotFound,
    FailedToReadOcrRegister
}

impl From<embassy_nrf::spim::Error> for SDError {
    fn from(error: embassy_nrf::spim::Error) -> Self {
        Self::SPIError(error)
    }
}

pub struct SDCard<'s, SPI, CS>
where
    SPI: embassy_nrf::spim::Instance,
    CS: embassy_nrf::gpio::Pin,
{
    spi: Spim<'s, SPI>,
    cs: Output<'s, CS>,
}

impl<'s, SPI, CS> SDCard<'s, SPI, CS>
where
    SPI: embassy_nrf::spim::Instance,
    CS: embassy_nrf::gpio::Pin,
{
    pub fn new(spi: Spim<'s, SPI>, cs: Output<'s, CS>) -> Self {
        Self { spi, cs }
    }

    pub async fn acquire(&mut self) -> Result<(), SDError> {
        // Bring card into SPI mode
        trace!("Enabling SPI mode");
        self.cs.set_high();
        for _ in 0..100 {
            let tx = [0xFF];
            self.spi.write(&tx).await?;
        }
        self.cs.set_low();

        let mut attempts = 100;
        while attempts > 0 {
            trace!("Attempting connection (round #{})", 32i32 - attempts);
            match self.card_command(CMD0, 0).await {
                Err(SDError::TimeoutCommand(CMD0)) => {
                    // Try again?
                    warn!("Timed out, trying again..");
                    attempts -= 1;
                }
                Err(e) => {
                    return Err(e);
                }
                Ok(R1_IDLE_STATE) => {
                    break;
                }
                Ok(r) => {
                    // Try again
                    warn!("Got response: {:x}, trying again..", r);
                }
            }

            Timer::after(Duration::from_millis(250)).await;
        }

        if attempts == 0 {
            return Err(SDError::CardNotFound);
        }

        trace!("Enabling CRC mode");
        if self.card_command(CMD59, 1).await? != R1_IDLE_STATE {
            warn!("Failed to enable CRC mode");
        }

        // Check card version
        trace!("Checking card version");
        let mut v1 = true;
        loop {
            if self.card_command(CMD8, 0x1AA).await? == (R1_ILLEGAL_COMMAND | R1_IDLE_STATE) {
                // We have a V1 card
                break;
            }
            let tx = [];
            let mut rx = [0; 4];
            self.spi.transfer(&mut rx, &tx).await?;
            if rx[3] == 0xAA {
                // We have a V2 card
                v1 = false;
                break;
            }
            Timer::after(Duration::from_millis(250)).await;
        }
        defmt::debug!("Card version: {:?}", if v1 { "v1" } else { "v2/SDHC" });

        trace!("Waiting for card to be ready");
        let mut ready_timeout = 32;
        let arg = if v1 { 0 } else { 0x4000_0000 };
        while self.card_acmd(ACMD41, arg).await? != R1_READY_STATE {
            Timer::after(Duration::from_millis(250)).await;
            ready_timeout -= 1;
            if ready_timeout == 0 {
                return Err(SDError::TimeoutCommand(ACMD41));
            }
        }

        if !v1 {
            trace!("Checking subversion");
            if self.card_command(CMD58, 0).await? != 0 {
                return Err(SDError::FailedToReadOcrRegister);
            }
            let tx = [];
            let mut rx = [0; 4];
            self.spi.transfer(&mut rx, &tx).await?;

            if (rx[0] & 0xC0) == 0xC0 {
                defmt::debug!("Card subversion: SDHC/SDXC");
            } else {
                defmt::debug!("Card subversion: SDSC");
            }
        }

        trace!("Setting CS to high");
        self.cs.set_high();

        Ok(())
    }

    /// Perform an application-specific command.
    async fn card_acmd(&mut self, command: u8, arg: u32) -> Result<u8, SDError> {
        self.card_command(CMD55, 0).await?;
        self.card_command(command, arg).await
    }

    async fn card_command(&mut self, command: u8, arg: u32) -> Result<u8, SDError> {
        self.not_busy().await?;

        trace!("Executing SD Command (cmd={},arg={})", command, arg);

        let mut buf = [
            0x40 | command,
            (arg >> 24) as u8,
            (arg >> 16) as u8,
            (arg >> 8) as u8,
            arg as u8,
            0,
        ];
        buf[5] = crc7(&buf[0..5]);

        self.spi.write(&buf).await?;

        // skip stuff byte for stop read
        if command == CMD12 {
            let mut _discarded_result = [0];
            self.spi.read(&mut _discarded_result).await?;
        }

        for i in 0..512 {
            // trace!("Awaiting command response (round #{})", i);
            let mut result = [0; 1];
            let tx = [];
            self.spi.transfer(&mut result, &tx).await?;
            if (result[0] & 0x80) == ERROR_OK {
                return Ok(result[0]);
            }
            Timer::after(Duration::from_millis(250)).await;
        }

        Err(SDError::TimeoutCommand(command))
    }

    async fn not_busy(&mut self) -> Result<(), SDError> {
        for i in 0..100 {
            trace!("Awaiting not busy (round #{})", i);
            let mut buf = [0; 1];
            let tx = [];
            self.spi.transfer(&mut buf, &tx).await?;
            if buf[0] == 0xFF {
                trace!("Card not busy.");
                return Ok(());
            }
            Timer::after(Duration::from_millis(250)).await;
        }

        trace!("Timeout waiting for not busy condition.");
        Err(SDError::TimeoutWaitNoBusy)
    }
}

// Possible errors the SD card can return

/// Card indicates last operation was a success
pub const ERROR_OK: u8 = 0x00;

// SD Card Commands

/// GO_IDLE_STATE - init card in spi mode if CS low
pub const CMD0: u8 = 0x00;
/// SEND_IF_COND - verify SD Memory Card interface operating condition.*/
pub const CMD8: u8 = 0x08;
/// SEND_CSD - read the Card Specific Data (CSD register)
pub const CMD9: u8 = 0x09;
/// STOP_TRANSMISSION - end multiple block read sequence
pub const CMD12: u8 = 0x0C;
/// SEND_STATUS - read the card status register
pub const CMD13: u8 = 0x0D;
/// READ_SINGLE_BLOCK - read a single data block from the card
pub const CMD17: u8 = 0x11;
/// READ_MULTIPLE_BLOCK - read a multiple data blocks from the card
pub const CMD18: u8 = 0x12;
/// WRITE_BLOCK - write a single data block to the card
pub const CMD24: u8 = 0x18;
/// WRITE_MULTIPLE_BLOCK - write blocks of data until a STOP_TRANSMISSION
pub const CMD25: u8 = 0x19;
/// APP_CMD - escape for application specific command
pub const CMD55: u8 = 0x37;
/// READ_OCR - read the OCR register of a card
pub const CMD58: u8 = 0x3A;
/// CRC_ON_OFF - enable or disable CRC checking
pub const CMD59: u8 = 0x3B;
/// SD_SEND_OP_COMD - Sends host capacity support information and activates
/// the card's initialization process
pub const ACMD41: u8 = 0x29;

/// status for card in the ready state
pub const R1_READY_STATE: u8 = 0x00;

/// status for card in the idle state
pub const R1_IDLE_STATE: u8 = 0x01;

/// status bit for illegal command
pub const R1_ILLEGAL_COMMAND: u8 = 0x04;

/// start data token for read or write single block*/
pub const DATA_START_BLOCK: u8 = 0xFE;

/// stop token for write multiple blocks*/
pub const STOP_TRAN_TOKEN: u8 = 0xFD;

/// start data token for write multiple blocks*/
pub const WRITE_MULTIPLE_TOKEN: u8 = 0xFC;

/// mask for data response tokens after a write block operation
pub const DATA_RES_MASK: u8 = 0x1F;

/// write data accepted token
pub const DATA_RES_ACCEPTED: u8 = 0x05;

/// Perform the 7-bit CRC used on the SD card
pub fn crc7(data: &[u8]) -> u8 {
    let mut crc = 0u8;
    for mut d in data.iter().cloned() {
        for _bit in 0..8 {
            crc <<= 1;
            if ((d & 0x80) ^ (crc & 0x80)) != 0 {
                crc ^= 0x09;
            }
            d <<= 1;
        }
    }
    (crc << 1) | 1
}
