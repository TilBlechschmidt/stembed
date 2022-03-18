#![no_std]
#![no_main]
#![feature(alloc_error_handler)]

extern crate alloc;
mod constants;

use defmt_rtt as _; // global logger
use nrf52840_hal as _; // memory layout
use panic_probe as _; // global panic handler

use alloc_cortex_m::CortexMHeap;
use constants::HEAP_SIZE;
use core::alloc::Layout;
use embassy::time::Instant;

#[global_allocator]
static ALLOCATOR: CortexMHeap = CortexMHeap::empty();

// same panicking *behavior* as `panic-probe` but doesn't print a panic message
// this prevents the panic message being printed *twice* when `defmt::panic` is invoked
#[defmt::panic_handler]
fn panic() -> ! {
    cortex_m::asm::udf()
}

#[alloc_error_handler]
fn oom(_: Layout) -> ! {
    defmt::panic!("We ran out of memory :(");
}

defmt::timestamp!("{=u64}", Instant::now().as_millis());

/// Initializes the global allocator and reserves the required memory
pub fn setup_heap() {
    {
        use core::mem::MaybeUninit;
        static mut HEAP: [MaybeUninit<u8>; HEAP_SIZE] = [MaybeUninit::uninit(); HEAP_SIZE];
        unsafe { ALLOCATOR.init((&mut HEAP).as_ptr() as usize, HEAP_SIZE) }
    }
}

/// Terminates the application and makes `probe-run` exit with exit-code = 0
pub fn exit() -> ! {
    loop {
        cortex_m::asm::bkpt();
    }
}
