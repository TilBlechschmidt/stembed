use std::time::Duration;

use alloc::boxed::Box;
use serialport::ClearBuffer;

mod gemini;
pub use gemini::*;

pub struct SerialPort(Box<dyn serialport::SerialPort>);

impl SerialPort {
    pub fn new(path: impl AsRef<str>) -> std::io::Result<Self> {
        let raw_port = serialport::new(path.as_ref(), 115_200)
            .timeout(Duration::MAX)
            .open()?;

        Self::new_raw(raw_port)
    }

    pub fn new_raw(raw_port: Box<dyn serialport::SerialPort>) -> std::io::Result<Self> {
        raw_port.clear(ClearBuffer::Input)?;
        Ok(Self(raw_port))
    }

    fn read_u8(&mut self) -> std::io::Result<u8> {
        let mut buffer = [0u8];
        let mut bytes_read = 0;

        while bytes_read < 1 {
            // TODO Check if the connection is up, potentially reconnect if possible
            bytes_read = self.0.read(&mut buffer)?;
        }

        Ok(buffer[0])
    }
}
