use core::cell::RefCell;

use defmt::{trace, warn};
use embassy::time::{Duration, Timer};
use embassy_nrf::{gpio::Output, spim::Spim};
use fat32::{Block, BlockDevice, BlockDeviceError, BlockID};

/// How many times a command shall be retried if the error is recoverable
const COMMAND_RETRY_COUNT: usize = 32 * 512;
const RESET_RETRY_COUNT: usize = 128 * 512;
const INIT_WAIT_COUNT: usize = 512 * 512;
const NOT_BUSY_WAIT_COUNT: usize = 1024 * 512;

const CHECK_PATTERN: u8 = 0xAA;
const TOKEN_START_BLOCK_SINGLE: u8 = 0b11111110;

#[derive(defmt::Format, Debug)]
pub enum SDError {
    SPIError(embassy_nrf::spim::Error),
    Timeout,
    CardNotFound,
    FailedToReadOcrRegister,
    ReadError(ReadError),
    R1Failure(R1Error),
    R7Failure(R7Error),
}

impl From<embassy_nrf::spim::Error> for SDError {
    fn from(error: embassy_nrf::spim::Error) -> Self {
        Self::SPIError(error)
    }
}

impl From<R1Error> for SDError {
    fn from(error: R1Error) -> Self {
        Self::R1Failure(error)
    }
}

impl From<R7Error> for SDError {
    fn from(error: R7Error) -> Self {
        Self::R7Failure(error)
    }
}

pub struct SDCard<'i, 's, SPI, CS>
where
    SPI: embassy_nrf::spim::Instance,
    CS: embassy_nrf::gpio::Pin,
{
    interface: RefCell<&'i mut SDCardInterface<'s, SPI, CS>>,
    version: CardVersion,
    capacity_class: CapacityClass,
}

pub struct SDCardInterface<'s, SPI, CS>
where
    SPI: embassy_nrf::spim::Instance,
    CS: embassy_nrf::gpio::Pin,
{
    spi: Spim<'s, SPI>,
    cs: Output<'s, CS>,
}

impl<'s, SPI, CS> SDCardInterface<'s, SPI, CS>
where
    SPI: embassy_nrf::spim::Instance,
    CS: embassy_nrf::gpio::Pin,
{
    pub fn new(spi: Spim<'s, SPI>, cs: Output<'s, CS>) -> Self {
        Self { spi, cs }
    }

    fn cs_assert(&mut self, asserted: bool) {
        if asserted {
            self.cs.set_low();
        } else {
            self.cs.set_high();
        }
    }

    async fn not_busy(&mut self) -> Result<(), SDError> {
        for i in 0..NOT_BUSY_WAIT_COUNT {
            trace!("Awaiting not busy (round #{})", i);
            let mut buf = [0xFF; 1];
            let tx = [];
            self.spi.transfer(&mut buf, &tx).await?;
            if buf[0] == 0xFF {
                trace!("Card not busy.");
                return Ok(());
            }
            // Timer::after(Duration::from_millis(10)).await;
        }

        trace!("Timeout waiting for not busy condition.");
        Err(SDError::Timeout)
    }

    async fn exec(&mut self, command: u8, argument: u32) -> Result<(), R1Error> {
        debug_assert_eq!(command & 0b11000000, 0);
        // trace!("exec {}", command);

        let mut buf = [
            // Set start & transmission bit and use the command index
            0b01_000000 | command,
            // Transmit the argument
            (argument >> 24) as u8,
            (argument >> 16) as u8,
            (argument >> 8) as u8,
            argument as u8,
            // Keep a reserved byte for the CRC
            0,
        ];

        // Calculate the CRC7 and add the trailing end bit
        buf[5] = (crc7(&buf[0..5]) << 1) | 1;

        // TODO Remove unwrap
        self.spi.write(&mut buf).await.unwrap();

        // Try the command up to `COMMAND_RETRY_COUNT` times if the error is recoverable
        for _ in 0..COMMAND_RETRY_COUNT {
            // Receive one byte (R1 response)
            let mut response = [0xFF; 1];
            self.spi.transfer_in_place(&mut response).await.unwrap();
            let result = R1Error::new(response[0]);
            // defmt::trace!("\t-\t-RECV: {:?} -> {:?}", response[0], result.as_ref());

            // Evaluate the result
            match result {
                Ok(_) => return Ok(()),
                Err(error) => {
                    match error {
                        R1Error::ChecksumMismatch => {
                            // Resend the command if the checksum was wrong
                            self.spi.write(&mut buf).await.unwrap();
                        }
                        R1Error::InvalidResponse => {
                            // The card is not ready yet, give it a moment
                            // Timer::after(Duration::from_millis(1)).await;
                            continue;
                        }
                        R1Error::Initializing => {
                            if command == 0 || command == 41 {
                                return Err(R1Error::Initializing);
                            } else {
                                return Ok(());
                            }
                        }
                        error => return Err(error),
                    }
                }
            }
        }

        Err(R1Error::InvalidResponse)
    }

    async fn exec_acmd(&mut self, command: u8, argument: u32) -> Result<(), R1Error> {
        // defmt::debug!("Executing ACMD{}", command);
        self.cs_assert(true);
        self.exec(55, 0).await?;
        self.exec(command, argument).await?;
        self.cs_assert(false);
        Ok(())
    }

    async fn exec_cmd(&mut self, command: u8, argument: u32) -> Result<(), R1Error> {
        // defmt::debug!("Executing CMD{}", command);
        self.cs_assert(true);

        if let Err(e) = self.not_busy().await {
            defmt::error!("BusyWaitErr: {}", e);
        }

        self.exec(command, argument).await?;
        self.cs_assert(false);
        Ok(())
    }

    async fn reset_card(&mut self) -> Result<(), SDError> {
        // Execute CMD0 a couple of times until the card responds (or give up)
        for _ in 0..RESET_RETRY_COUNT {
            // defmt::debug!("\tReset round #{}", i);

            self.cs_assert(true);
            let response = self.exec(0, 0).await;
            self.cs_assert(false);

            match response {
                // We expect the card to be in an uninitialized state!
                Ok(_) => continue,
                Err(R1Error::Initializing) => return Ok(()),
                Err(error) => return Err(error.into()),
            }
        }

        Err(SDError::Timeout)
    }

    async fn check_card_version(&mut self) -> Result<CardVersion, SDError> {
        self.cs_assert(true);
        let response = self.exec(8, (1 << 8) | (CHECK_PATTERN as u32)).await;

        match response {
            Ok(_) => {
                // Read R7 response
                let mut response = [0xFF; 4];
                self.spi.transfer_in_place(&mut response).await?;
                self.cs_assert(false);
                defmt::trace!("R7 {:?}", response);

                R7Error::new(response, CHECK_PATTERN)
                    .map(|_| CardVersion::V2)
                    .map_err(SDError::from)
            }
            Err(R1Error::IllegalCommand) => Ok(CardVersion::V1),
            Err(error) => Err(error.into()),
        }
    }

    async fn initialize_card(&mut self, version: CardVersion) -> Result<(), SDError> {
        let argument = match version {
            CardVersion::V1 => 0,
            CardVersion::V2 => 1 << 30,
        };

        for i in 0..INIT_WAIT_COUNT {
            defmt::debug!("\tInit round #{}", i);

            match self.exec_acmd(41, argument).await {
                Ok(()) => break,
                Err(R1Error::Initializing) => continue,
                Err(error) => return Err(error.into()),
            }
        }

        Ok(())
    }

    async fn check_card_capacity_class(&mut self) -> Result<CapacityClass, SDError> {
        self.cs_assert(true);
        self.exec(58, 0).await?;

        let mut ocr = [0xFF; 4];
        self.spi.transfer_in_place(&mut ocr).await?;
        self.cs_assert(false);

        if (ocr[0] & 0xC0) == 0xC0 {
            Ok(CapacityClass::High)
        } else {
            Ok(CapacityClass::Standard)
        }
    }

    pub async fn acquire(&mut self) -> Result<SDCard<'_, 's, SPI, CS>, SDError> {
        // Unassert CS and wait a while for the card to sort itself out in case we just booted up
        defmt::debug!("Bringing card into idle state");
        self.cs_assert(true);
        let delay = [0xFF; 100];
        self.spi.write(&delay).await?;
        self.cs_assert(false);
        self.spi.write(&[0xFF; 2]).await?;

        // Reset the card and put it into SPI mode
        defmt::debug!("Resetting card");
        self.reset_card().await?;

        // Enable CRC checking
        defmt::debug!("Activating CRC check");
        self.cs_assert(true);
        self.exec(59, 1).await?;
        self.cs_assert(false);

        // Check which version this card is and verify the operating voltage
        defmt::debug!("Checking card version");
        let version = self.check_card_version().await?;

        // Prepare the card for operation and wait for initialization to complete
        defmt::debug!("Initializing card");
        self.initialize_card(version).await?;

        // Read the card type for later reference
        defmt::debug!("Reading capacity class");
        let capacity_class = if version == CardVersion::V1 {
            CapacityClass::Standard
        } else {
            self.check_card_capacity_class().await?
        };

        // Make sure the block length is 512-byte (V1 cards can have shorter blocks by default)
        if version == CardVersion::V1 {
            defmt::debug!("Setting block size");
            self.exec_cmd(16, 512).await?;
        }

        self.cs_assert(false);

        Ok(SDCard {
            interface: RefCell::new(self),
            capacity_class,
            version,
        })
    }

    /// Reads a number of bytes from the given address.
    /// Note that the usage of the address is dependent on the card type.
    /// SDHC cards use block addresses while SDSC cards use byte addresses!
    async fn read_block(&mut self, address: u32) -> Result<[u8; 512], SDError> {
        // defmt::debug!("READ BLOCK START");
        self.cs_assert(true);
        self.exec(17, address).await?;

        // Read and check the response token
        // defmt::debug!("READ BLOCK WAIT");
        let mut token = [0xFF];
        // let mut token_read_count = 0;
        while token[0] != TOKEN_START_BLOCK_SINGLE {
            self.spi.transfer_in_place(&mut token).await?;

            // trace!("token: {}", token[0]);

            // if token[0] != TOKEN_START_BLOCK_SINGLE {
            //     // TODO Parse the data error token or throw InvalidResponse if it isn't one
            //     return Err(SDError::ReadError(ReadError::InvalidResponse));
            // }

            // Timer::after(Duration::from_millis(250)).await;
            // token_read_count += 1;
        }

        // Read the data block
        // defmt::debug!("READ BLOCK READ (token iterations = {})", token_read_count);
        let mut block = [0xFF; 512];
        self.spi.transfer_in_place(&mut block).await?;

        // Read and verify the CRC
        // defmt::debug!("READ BLOCK CRC");
        let mut raw_crc = [0xFF; 2];
        self.spi.transfer_in_place(&mut raw_crc).await?;
        let received_crc = u16::from_be_bytes(raw_crc);
        let calculated_crc = crc16(&block);

        self.cs_assert(false);

        // defmt::debug!("READ BLOCK DONE");
        if received_crc != calculated_crc {
            return Err(SDError::ReadError(ReadError::ChecksumMismatch));
        }

        Ok(block)
    }
}

impl<'i, 's, SPI, CS> SDCard<'i, 's, SPI, CS>
where
    SPI: embassy_nrf::spim::Instance,
    CS: embassy_nrf::gpio::Pin,
{
    pub async fn read_block(&self, block_address: u32) -> Result<[u8; 512], SDError> {
        // defmt::info!("Reading block #{}", block_address);
        let mut interface = self.interface.borrow_mut();

        if self.capacity_class == CapacityClass::Standard {
            debug_assert!(
                block_address as u64 * 512 < u32::MAX as u64,
                "block address overflowed card capacity"
            );
            interface.read_block(block_address * 512).await
        } else {
            interface.read_block(block_address).await
        }
    }

    pub async fn read_block_fs(
        &self,
        block_id: BlockID,
    ) -> Result<Block, BlockDeviceError<SDError>> {
        self.read_block(block_id.into_inner())
            .await
            .map(Block::new)
            .map_err(|error| BlockDeviceError::DeviceError(error))
    }
}

#[derive(defmt::Format, Debug)]
pub enum ReadError {
    /// Received data block did not pass checksum verification
    ChecksumMismatch,
    /// Unexpected response token received
    InvalidResponse,
}

#[derive(defmt::Format, PartialEq, Eq, Clone, Copy)]
pub enum CapacityClass {
    /// SDSC cards
    Standard,
    /// SDHC or SDXC cards
    High,
}

#[derive(defmt::Format, PartialEq, Eq, Clone, Copy)]
pub enum CardVersion {
    V1,
    V2,
}

#[derive(defmt::Format, Debug)]
pub enum R7Error {
    /// The echoed check-pattern did not match the sent one
    InvalidCheckPattern,
    /// Host voltage is not supported by the card
    UnsupportedVoltage,
}

impl R7Error {
    fn new(response: [u8; 4], check_pattern: u8) -> Result<(), Self> {
        if response[3] != check_pattern {
            Err(Self::InvalidCheckPattern)
        } else {
            // TODO Check for invalid voltage
            Ok(())
        }
    }
}

#[derive(defmt::Format, Debug)]
pub enum R1Error {
    /// The card is in idle state and running the initializing process
    Initializing,
    /// An erase sequence was cleared before executing because an out of erase sequence command was received.
    EraseReset,
    /// An illegal command code was detected.
    IllegalCommand,
    /// The CRC check of the last command failed.
    ChecksumMismatch,
    /// An error in the sequence of erase commands occurred.
    EraseSequenceFailed,
    /// A misaligned address that did not match the block length was used in the command.
    MisalignedAddress,
    /// The command's argument (e.g. address, block length) was outside the allowed range for this card.
    OutOfBoundsAddress,
    /// More than one error condition occured (which according to the specification is not allowed)
    InvalidResponse,
}

impl R1Error {
    fn new(response: u8) -> Result<(), Self> {
        match response {
            0b00000001 => Err(Self::Initializing),
            0b00000010 => Err(Self::EraseReset),
            0b00000100 => Err(Self::IllegalCommand),
            0b00001000 => Err(Self::ChecksumMismatch),
            0b00010000 => Err(Self::EraseSequenceFailed),
            0b00100000 => Err(Self::MisalignedAddress),
            0b01000000 => Err(Self::OutOfBoundsAddress),
            0b00000000 => Ok(()),
            _ => Err(Self::InvalidResponse),
        }
    }
}

/// Perform the 7-bit CRC used on the SD card
fn crc7(data: &[u8]) -> u8 {
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
    crc
}

/// Perform the X25 CRC calculation, as used for data blocks.
pub fn crc16(data: &[u8]) -> u16 {
    let mut crc = 0u16;
    for &byte in data {
        crc = ((crc >> 8) & 0xFF) | (crc << 8);
        crc ^= u16::from(byte);
        crc ^= (crc & 0xFF) >> 4;
        crc ^= crc << 12;
        crc ^= (crc & 0xFF) << 5;
    }
    crc
}
