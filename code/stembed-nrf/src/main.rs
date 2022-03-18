#![no_std]
#![no_main]
#![feature(generic_associated_types)]
#![feature(type_alias_impl_trait)]

extern crate alloc;

use crate::firmware::{sdcard::SDCardInterface, KeymatrixInput, Reader};
use embassy::{executor::Spawner, time::Instant};
use embassy_nrf::{
    gpio::{Input, Level, Output, OutputDrive, Pin, Pull},
    interrupt,
    peripherals::{P0_13, SPI3},
    spim::{self, Spim},
    Peripherals,
};
use fat32::{FileReader, Filesystem};
use stembed::{
    core::{
        dict::BinaryDictionary,
        engine::Engine,
        processor::{
            text_formatter::{TextFormatter, TextOutputInstruction},
            CommandProcessor,
        },
        Stroke,
    },
    input::InputSource,
};
use stembed_nrf::{
    self as _, // global logger + panicking-behavior + memory layout
    exit,
    setup_heap,
};

mod firmware;

#[embassy::main]
async fn main(_spawner: Spawner, p: Peripherals) {
    setup_heap();
    test_stuff(p).await;
    exit();
}

async fn test_stuff(p: Peripherals) {
    let (mut input_source, spim, ncs) = build_io(p);
    defmt::info!("Plug in your card now!");
    embassy::time::Timer::after(embassy::time::Duration::from_millis(1000)).await;

    let mut interface = SDCardInterface::new(spim, ncs);
    let card = interface.acquire().await.unwrap();

    let filesystem = Filesystem::new(
        |address| card.read_block_fs(address),
        |_address, _block| async move {
            defmt::unimplemented!();
        },
    )
    .await
    .unwrap();

    let file = filesystem.find_file("DICT", "BIN").await.unwrap().unwrap();
    let mut file_reader = FileReader::new(file, &filesystem);
    file_reader.cache_fat().await.unwrap();
    let mut reader = Reader::new(file_reader);

    let dictionary = BinaryDictionary::new(&mut reader).await.unwrap();
    let mut engine = Engine::new(&dictionary);
    let mut formatter = TextFormatter::new();
    let context = dictionary.stroke_context();

    use alloc::string::ToString;

    loop {
        dictionary.reset_lookup_count();
        let input = input_source.scan().unwrap();
        defmt::debug!("Received input");
        let stroke = Stroke::from_input(input, &KeymatrixInput::DEFAULT_KEYMAP, &context);
        defmt::debug!("Processing stroke: {}", stroke.to_string().as_str());
        let delta = engine.push(stroke).await;
        let output = formatter.consume(delta);

        for instruction in output {
            match instruction {
                TextOutputInstruction::Backspace(count) => defmt::debug!("<bksp*{}>", count),
                TextOutputInstruction::Write(text) => {
                    let text_slice: &str = &text;
                    defmt::debug!("Text output: '{}'", text_slice)
                }
            }
        }
        defmt::debug!("Lookup count: {}\n", dictionary.lookup_count());
    }
}

fn build_io(
    p: Peripherals,
) -> (
    KeymatrixInput<'static>,
    Spim<'static, SPI3>,
    Output<'static, P0_13>,
) {
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

    let input = KeymatrixInput {
        columns_left,
        rows_left,
        columns_right,
        rows_right,
    };

    let cs = p.P0_13;
    let sck = p.P0_15;
    let mosi = p.P0_17;
    let miso = p.P0_20;

    let mut config = spim::Config::default();
    config.frequency = spim::Frequency::M32;

    let irq = interrupt::take!(SPIM3);
    let spim = spim::Spim::new(p.SPI3, irq, sck, miso, mosi, config);
    let ncs = Output::new(cs, Level::High, OutputDrive::Standard);

    (input, spim, ncs)
}
