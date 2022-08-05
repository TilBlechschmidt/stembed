#![no_std]
#![no_main]
#![macro_use]
#![feature(generic_associated_types)]
#![feature(type_alias_impl_trait)]
#![feature(array_try_map)]

use defmt_rtt as _; // global logger
use panic_probe as _; // global panic handler

defmt::timestamp!("{=u64}", embassy_executor::time::Instant::now().as_millis());

mod driver;
mod hardware;
mod setup;

// TODO There is the possiblity to set the keyboard language tag which gives the OS a hint on which keyboard layout to use!
// TODO Change asserts in hardware modules to returning a result so we can fail gracefully

// TODO By giving ownership of all the 'static stuff to the `Runtime`, we can keep borrows around and send the runtime into the task thus removing all the `static` madness.
//      This requires getting the implementations of say `Keyboard` by borrowing from the runtime before passing it into the task though.
//      However, at that stage we could only borrow it to the task which causes more problems because the borrow is not 'static :(
//      Two alternatives: 1. Dump the runtime into a Forever at the root level (which is more sensible than doing it in every hardware module)
//                        2. Do not use tasks but instead just select! all the runtimes at the end.
//                  At this point the second option does not seem all too bad tbh.

// TODO The IR allows us to make the whole engine stroke agnostic. Each dictionary can do an IR->Stroke mapping itself based on the system it was compiled with.
//      This also removes the stroke formatting logic (making it human readable) from the engine. It can now be contained in a fallback dictionary — this fbd could just follow english steno `–` logic but allow for customized letters; others could later be added as required
//
//      FBD can be fed from `const` (as default) or flash.
//      Which FBD is active is a choice by the user just as regular dicts are.
//      => Thus switching between systems becomes activating a different set of dicts+FBD (dictionary stacks could be the term)

#[embassy_executor::main]
async fn main(s: embassy_executor::executor::Spawner, p: embassy_nrf::Peripherals) {
    defmt::info!("Configuring peripherals");

    let peripherals = setup::peripherals(&s, p).await;
    runtime::Runtime::execute(peripherals).await;
}
