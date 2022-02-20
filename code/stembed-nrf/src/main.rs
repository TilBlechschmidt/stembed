#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]
#![feature(alloc_error_handler)]

extern crate alloc;

use alloc::string::ToString;
use alloc_cortex_m::CortexMHeap;
use core::alloc::Layout;
use core::sync::atomic::{AtomicUsize, Ordering};
pub use defmt::*;
use defmt_rtt as _; // global logger
use embassy::executor::Spawner;
use embassy_nrf::gpio::Pin;
use embassy_nrf::gpio::{Input, Pull};
use embassy_nrf::gpio::{Level, Output, OutputDrive};
use embassy_nrf::Peripherals;
use embassy_nrf::{interrupt, spim};
use panic_probe as _;
use stembed::core::processor::text_formatter::{TextFormatter, TextOutputInstruction};
use stembed::core::processor::CommandProcessor;
use stembed::core::{engine::Engine, Stroke, StrokeContext};
use stembed::input::InputSource;

#[global_allocator]
static ALLOCATOR: CortexMHeap = CortexMHeap::empty();

mod dict;
mod input;
mod sdcard;

use dict::DummyDictionary;
use input::KeymatrixInput;

#[embassy::main]
async fn main(_spawner: Spawner, p: Peripherals) {
    // {
    //     use core::mem::MaybeUninit;
    //     const HEAP_SIZE: usize = 65_536; // 64K
    //     static mut HEAP: [MaybeUninit<u8>; HEAP_SIZE] = [MaybeUninit::uninit(); HEAP_SIZE];
    //     unsafe { ALLOCATOR.init((&mut HEAP).as_ptr() as usize, HEAP_SIZE) }
    // }

    // test_engine(p);
    test_sdcard(p).await;
}

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

    let mut card = sdcard::SDCard::new(spim, ncs);
    let result = card.acquire().await;
    warn!("Acquire card result: {:?}", result);
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
        let mut engine = Engine::new(DummyDictionary);
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
            let stroke =
                Stroke::from_input(input, &KeymatrixInput::DEFAULT_KEYMAP, context.clone());
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

async fn test_input(p: Peripherals) {
    // let mut led1 = Output::new(p.P1_10, Level::Low, OutputDrive::Standard);
    // let mut led2 = Output::new(p.P1_04, Level::Low, OutputDrive::Standard);

    // led1.set_high();
    // led2.set_high();
}

#[alloc_error_handler]
fn oom(_: Layout) -> ! {
    defmt::panic!("We ran out of memory :(");
}

defmt::timestamp! {"{=u64}", {
        static COUNT: AtomicUsize = AtomicUsize::new(0);
        COUNT.fetch_add(1, Ordering::Relaxed) as u64
    }
}
