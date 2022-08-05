use core::sync::atomic::{AtomicBool, Ordering};
use defmt::{debug, unwrap};
use embassy_nrf::{
    interrupt::{self},
    peripherals::USBD,
    usb::{self, Driver, PowerUsb, UsbSupply},
};
use embassy_usb::{Builder, Config, DeviceStateHandler};
use embassy_util::{channel::signal::Signal, select, Either, Forever};

pub mod channel;
pub mod keyboard;

/// Asks the remote end to reactivate the USB connection
static REMOTE_WAKEUP: Signal<()> = Signal::new();

static SUSPENDED: AtomicBool = AtomicBool::new(false);
static CONFIGURED: AtomicBool = AtomicBool::new(false);

static DEVICE_DESCRIPTOR: Forever<[u8; 256]> = Forever::new();
static CONFIG_DESCRIPTOR: Forever<[u8; 256]> = Forever::new();
static BOS_DESCRIPTOR: Forever<[u8; 256]> = Forever::new();
static CONTROL_BUF: Forever<[u8; 64]> = Forever::new();

static STATE_HANDLER: GlobalStateHandler = GlobalStateHandler;

pub struct UsbBus;

impl UsbBus {
    pub fn wake_up() {
        REMOTE_WAKEUP.signal(());
    }

    pub fn is_suspended() -> bool {
        SUSPENDED.load(Ordering::Acquire)
    }

    pub fn is_configured() -> bool {
        CONFIGURED.load(Ordering::Acquire)
    }
}

pub fn configure<P: UsbSupply + 'static>(
    usbd: USBD,
    config: Config<'static>,
    supply: P,
) -> Builder<Driver<USBD, P>> {
    super::clock::enable_high_frequency_oscillator();

    let driver = usb::Driver::new(usbd, interrupt::take!(USBD), supply);

    Builder::new(
        driver,
        config,
        DEVICE_DESCRIPTOR.put([0; 256]),
        CONFIG_DESCRIPTOR.put([0; 256]),
        BOS_DESCRIPTOR.put([0; 256]),
        CONTROL_BUF.put([0; 64]),
        Some(&STATE_HANDLER),
    )
}

#[embassy_executor::task]
pub async fn run(runtime: Builder<'static, Driver<'static, USBD, PowerUsb>>) {
    let mut usb = runtime.build();

    loop {
        usb.run_until_suspend().await;
        match select(usb.wait_resume(), REMOTE_WAKEUP.wait()).await {
            Either::First(_) => (),
            Either::Second(_) => unwrap!(usb.remote_wakeup().await),
        }
    }
}

struct GlobalStateHandler;

impl DeviceStateHandler for GlobalStateHandler {
    fn enabled(&self, enabled: bool) {
        CONFIGURED.store(false, Ordering::Relaxed);
        SUSPENDED.store(false, Ordering::Release);
        if enabled {
            debug!("Device enabled");
        } else {
            debug!("Device disabled");
        }
    }

    fn reset(&self) {
        CONFIGURED.store(false, Ordering::Relaxed);
        debug!("Bus reset, the Vbus current limit is 100mA");
    }

    fn addressed(&self, addr: u8) {
        CONFIGURED.store(false, Ordering::Relaxed);
        debug!("USB address set to: {}", addr);
    }

    fn configured(&self, configured: bool) {
        CONFIGURED.store(configured, Ordering::Relaxed);
        if configured {
            debug!(
                "Device configured, it may now draw up to the configured current limit from Vbus."
            )
        } else {
            debug!("Device is no longer configured, the Vbus current limit is 100mA.");
        }
    }

    fn suspended(&self, suspended: bool) {
        if suspended {
            debug!("Device suspended, the Vbus current limit is 500µA (or 2.5mA for high-power devices with remote wakeup enabled).");
            SUSPENDED.store(true, Ordering::Release);
        } else {
            SUSPENDED.store(false, Ordering::Release);
            if CONFIGURED.load(Ordering::Relaxed) {
                debug!(
                    "Device resumed, it may now draw up to the configured current limit from Vbus"
                );
            } else {
                debug!("Device resumed, the Vbus current limit is 100mA");
            }
        }
    }
}
