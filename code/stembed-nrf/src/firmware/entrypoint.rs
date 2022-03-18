
use alloc::string::{String, ToString};
use alloc_cortex_m::CortexMHeap;
use core::alloc::Layout;
use core::sync::atomic::{AtomicUsize, Ordering};
pub use defmt::*;
use embassy::executor::Spawner;
use embassy_nrf::gpio::Pin;
use embassy_nrf::gpio::{Input, Pull};
use embassy_nrf::gpio::{Level, Output, OutputDrive};
use embassy_nrf::Peripherals;
use embassy_nrf::{interrupt, spim};
use fat32::{Block, BlockDeviceError, BlockID, FileReader, Filesystem, FilesystemError};
use futures::{Future, StreamExt};
use stembed::core::processor::text_formatter::{TextFormatter, TextOutputInstruction};
use stembed::core::processor::CommandProcessor;
use stembed::core::{engine::Engine, Stroke, StrokeContext};
use stembed::input::InputSource;
use stembed::io::Read;


mod dict;
mod input;
mod sdcard;

use dict::DummyDictionary;
use input::KeymatrixInput;

mod sdcard_old;

async fn test_sdcard(p: Peripherals) {
    let cs = p.P0_13;
    let sck = p.P0_15;
    let mosi = p.P0_17;
    let miso = p.P0_20;

    let mut config = spim::Config::default();
    config.frequency = spim::Frequency::M32;

    let irq = interrupt::take!(SPIM3);
    let spim = spim::Spim::new(p.SPI3, irq, sck, miso, mosi, config);
    let ncs = Output::new(cs, Level::High, OutputDrive::Standard);

    info!("Plug in your card now!");
    embassy::time::Timer::after(embassy::time::Duration::from_millis(5000)).await;

    // let mut interface = sdcard_old::SDCard::new(spim, ncs);
    // let result = interface.acquire().await;
    // warn!("Acquire card result: {:?}", result.as_ref().err());

    let mut interface = sdcard::SDCardInterface::new(spim, ncs);
    let result = interface.acquire().await;
    warn!("Acquire card result: {:?}", result.as_ref().err());

    if let Ok(card) = result {
        let filesystem = Filesystem::new(
            |address| card.read_block_fs(address),
            |_address, _block| async move {
                defmt::unimplemented!();
            },
        )
        .await;

        warn!("Filesystem init result: {:?}", filesystem.as_ref().err());

        if let Ok(filesystem) = filesystem {
            let stream = filesystem.enumerate_directory(filesystem.root_directory());
            futures::pin_mut!(stream);

            while let Some(entry) = stream.next().await {
                match entry {
                    Ok(entry) => {
                        info!("{:?}", entry);
                    }
                    Err(error) => {
                        info!("{:?}", error);
                        break;
                    }
                }
            }

            let file = filesystem.find_file("HELLO", "TXT").await.unwrap().unwrap();
            info!("FOUND FILE: {:?}", file);

            let file_size = file.size();
            let mut reader = FileReader::new(file, &filesystem);

            let mut content = String::new();
            for i in 0..file_size {
                let byte = reader.read(i).await.unwrap();
                content.push(byte as char);
            }
            info!("File content: '{}'", content.as_str());
        }
    }
}

pub struct Reader<'f, E, RFut, RFn, WFut, WFn>
where
    RFut: Future<Output = Result<Block, BlockDeviceError<E>>>,
    RFn: Fn(BlockID) -> RFut,
    WFut: Future<Output = Result<(), BlockDeviceError<E>>>,
    WFn: Fn(BlockID, Block) -> WFut,
{
    file: FileReader<'f, E, RFut, RFn, WFut, WFn>,
    offset: u32,
}

impl<'f, E, RFut, RFn, WFut, WFn> Read for Reader<'f, E, RFut, RFn, WFut, WFn>
where
    RFut: Future<Output = Result<Block, BlockDeviceError<E>>>,
    RFn: Fn(BlockID) -> RFut,
    WFut: Future<Output = Result<(), BlockDeviceError<E>>>,
    WFn: Fn(BlockID, Block) -> WFut,
{
    fn read_u8(&mut self) -> Result<u8, stembed::io::Error> {
        let data = self.file.read(self.offset).await;
        self.offset += 1;
        data.map_err(|_e| stembed::io::Error::Unknown)
    }
}

fn test_engine(p: Peripherals) {
    warn!("Testing engine!");
    debug!("Heap usage: {} bytes", ALLOCATOR.used());

    {
        info!("Creating context");
        let context = StrokeContext::new("#STKPWHR", "AO*EU", "FRPBLGTSDZ", &[])
            .expect("default stroke context");
        debug!("Heap usage: {} bytes", ALLOCATOR.used());

        info!("Creating engine");
        let dictionary = DummyDictionary::default();
        let mut engine = Engine::new(&dictionary);
        debug!("Heap usage: {} bytes", ALLOCATOR.used());

        info!("Creating formatter");
        let mut formatter = TextFormatter::new();
        debug!("Heap usage: {} bytes", ALLOCATOR.used());

        let columns_left = [
            p.P0_07.degrade(),
            p.P0_08.degrade(),
            p.P1_09.degrade(),
            p.P1_06.degrade(),
            p.P0_12.degrade(),
            p.P0_04.degrade(), // Pin or keyboard column currently broken
        ]
        .map(|pin| Output::new(pin, Level::Low, OutputDrive::Standard));

        let rows_left = [p.P0_24.degrade(), p.P0_22.degrade(), p.P0_05.degrade()]
            .map(|pin| Input::new(pin, Pull::Down));

        let columns_right = [
            p.P1_13.degrade(),
            p.P0_06.degrade(),
            p.P0_26.degrade(),
            p.P0_29.degrade(),
            p.P0_02.degrade(),
            p.P0_30.degrade(),
        ]
        .map(|pin| Output::new(pin, Level::Low, OutputDrive::Standard));

        let rows_right = [p.P0_28.degrade(), p.P0_03.degrade(), p.P1_11.degrade()]
            .map(|pin| Input::new(pin, Pull::Down));

        let mut input_source = KeymatrixInput {
            columns_left,
            rows_left,
            columns_right,
            rows_right,
        };

        loop {
            let input = input_source.scan().unwrap();
            debug!("Received input");
            let stroke = Stroke::from_input(input, &KeymatrixInput::DEFAULT_KEYMAP, &context);
            debug!("Processing stroke: {}", stroke.to_string().as_str());
            let delta = engine.push(stroke);
            let output = formatter.consume(delta);

            for instruction in output {
                match instruction {
                    TextOutputInstruction::Backspace(count) => debug!("<bksp*{}>", count),
                    TextOutputInstruction::Write(text) => {
                        let text_slice: &str = &text;
                        debug!("Text output: '{}'", text_slice)
                    }
                }
            }

            debug!("Heap usage: {} bytes", ALLOCATOR.used());
        }
    }
}

// async fn test_input(p: Peripherals) {
// let mut led1 = Output::new(p.P1_10, Level::Low, OutputDrive::Standard);
// let mut led2 = Output::new(p.P1_04, Level::Low, OutputDrive::Standard);
//
// led1.set_high();
// led2.set_high();
// }
