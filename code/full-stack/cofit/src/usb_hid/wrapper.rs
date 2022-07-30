// Code blatantly stolen from minidsp-rs — I'll just trust them that its safe for now :D
// https://github.com/mrene/minidsp-rs/blob/6c513df55715ffda168ce7135b3b8c9ab35cf162/minidsp/src/transport/hid/wrapper.rs

use hidapi::HidDevice;
use std::ops::Deref;

/// Wraps an underlying HidDevice, adding Sync+Send
pub struct HidDeviceWrapper {
    pub inner: HidDevice,
}

impl HidDeviceWrapper {
    pub fn new(inner: HidDevice) -> Self {
        HidDeviceWrapper { inner }
    }
}

impl Deref for HidDeviceWrapper {
    type Target = HidDevice;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

// hidapi's libusb backend is thread-safe, different threads can read + send and it does its own locking
unsafe impl Sync for HidDeviceWrapper {}
unsafe impl Send for HidDeviceWrapper {}
