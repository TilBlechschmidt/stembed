use core::{
    mem,
    sync::atomic::{AtomicBool, Ordering},
};
use defmt::{debug, info, unwrap, warn};
use embassy::{
    channel::signal::Signal,
    util::{select, select3, Either, Either3, Forever},
};
use embassy_nrf::{
    interrupt::{self, InterruptExt},
    pac,
    peripherals::USBD,
    usb::{self, Driver},
};
use embassy_usb::{Builder, Config, DeviceStateHandler};

pub mod channel;
pub mod keyboard;

/// Asks the remote end to reactivate the USB connection
static REMOTE_WAKEUP: Signal<()> = Signal::new();

static SUSPENDED: AtomicBool = AtomicBool::new(false);
static CONFIGURED: AtomicBool = AtomicBool::new(false);
static CONNECTED: AtomicBool = AtomicBool::new(false);

static VBUS_CONNECTED_NOTIFIER: Signal<bool> = Signal::new();

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

    pub fn is_connected() -> bool {
        CONNECTED.load(Ordering::Acquire)
    }
}

pub fn configure(usbd: USBD, config: Config<'static>) -> Builder<Driver<USBD>> {
    super::clock::enable_high_frequency_oscillator();
    configure_power_interrupt();

    let driver = usb::Driver::new(usbd, interrupt::take!(USBD));

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

#[embassy::task]
pub async fn run(runtime: Builder<'static, Driver<'static, USBD>>) {
    let mut usb = runtime.build();

    enable_command().await;
    loop {
        match select(usb.run_until_suspend(), VBUS_CONNECTED_NOTIFIER.wait()).await {
            Either::First(_) => {}
            Either::Second(enable) => {
                if enable {
                    warn!("Enable when already enabled!");
                } else {
                    usb.disable().await;
                    enable_command().await;
                }
            }
        }

        match select3(
            usb.wait_resume(),
            VBUS_CONNECTED_NOTIFIER.wait(),
            REMOTE_WAKEUP.wait(),
        )
        .await
        {
            Either3::First(_) => (),
            Either3::Second(enable) => {
                if enable {
                    warn!("Enable when already enabled!");
                } else {
                    usb.disable().await;
                    enable_command().await;
                }
            }
            Either3::Third(_) => unwrap!(usb.remote_wakeup().await),
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

fn configure_power_interrupt() {
    let power: pac::POWER = unsafe { mem::transmute(()) };
    let power_irq = interrupt::take!(POWER_CLOCK);

    power_irq.set_handler(on_power_interrupt);
    power_irq.unpend();
    power_irq.enable();

    power
        .intenset
        .write(|w| w.usbdetected().set().usbremoved().set());
}

fn on_power_interrupt(_: *mut ()) {
    let regs = unsafe { &*pac::POWER::ptr() };

    if regs.events_usbdetected.read().bits() != 0 {
        regs.events_usbdetected.reset();
        info!("Vbus detected, enabling USB...");
        VBUS_CONNECTED_NOTIFIER.signal(true);
        CONNECTED.store(true, Ordering::Release);
    }

    if regs.events_usbremoved.read().bits() != 0 {
        regs.events_usbremoved.reset();
        info!("Vbus removed, disabling USB...");
        VBUS_CONNECTED_NOTIFIER.signal(false);
        CONNECTED.store(false, Ordering::Release);
    }
}

async fn enable_command() {
    loop {
        if VBUS_CONNECTED_NOTIFIER.wait().await {
            break;
        } else {
            warn!("Received disable signal when already disabled!");
        }
    }
}
