use super::{MessageID, Transport};
use core::future::Future;
use hidapi::HidDevice;
use std::sync::{mpsc, Arc};
use tokio::sync::{
    broadcast::{self, error::RecvError},
    Mutex,
};

use wrapper::HidDeviceWrapper;

mod wrapper;

/// Transport implementation transferring data via USB HID
///
/// Uses [`hidapi`](https://docs.rs/hidapi/latest/hidapi/) under the hood. Spawns two threads upon initialization which will handle data transfer in the background.
#[doc(cfg(feature = "usb"))]
pub struct UsbHidTransport {
    tx: mpsc::Sender<[u8; 64]>,
    rx: Mutex<broadcast::Receiver<[u8; 64]>>,
}

impl UsbHidTransport {
    pub fn new(device: HidDevice) -> Self {
        let (tx, rx) = Self::spawn_communication_thread(device);
        Self {
            tx,
            rx: Mutex::new(rx),
        }
    }

    fn spawn_communication_thread(
        device: HidDevice,
    ) -> (mpsc::Sender<[u8; 64]>, broadcast::Receiver<[u8; 64]>) {
        // TODO Make sure the threads are cleaned up when the instance is dropped!

        let (hd_tx, hd_rx) = mpsc::channel::<[u8; 64]>();
        let (dh_tx, dh_rx) = broadcast::channel(64_000);

        device
            .set_blocking_mode(true)
            .expect("failed to set device to blocking mode");

        let device_tx = Arc::new(HidDeviceWrapper::new(device));
        let device_rx = device_tx.clone();

        std::thread::spawn(move || {
            while let Ok(packet) = hd_rx.recv() {
                if let Err(e) = device_tx.write(&packet) {
                    eprintln!("failed to send packet to USB device {e:?}");
                }
            }

            eprintln!("host-device writer thread exited");
        });

        std::thread::spawn(move || loop {
            let mut buf = [0; 64];
            if let Err(e) = device_rx.read(&mut buf) {
                eprintln!("failed to receive packet from USB device {e:?}");
            } else {
                dh_tx.send(buf).expect("failed to forward received packet");
            }
        });

        (hd_tx, dh_rx)
    }
}

impl Transport<63> for UsbHidTransport {
    type TxFut<'t> = impl Future<Output = ()> + 't
    where
        Self: 't;

    type RxFut<'t> = impl Future<Output = (MessageID, [u8; 63])> + 't
    where
        Self: 't;

    fn send<'t>(&'t self, id: MessageID, data: [u8; 63]) -> Self::TxFut<'t> {
        let mut packet = [0; 64];
        packet[0] = id;
        packet[1..].copy_from_slice(&data);

        async move {
            self.tx
                .send(packet)
                .expect("failed to forward packet to send thread")
        }
    }

    fn recv<'t>(&'t self) -> Self::RxFut<'t> {
        async move {
            loop {
                match self.rx.lock().await.recv().await {
                    Ok(packet) => {
                        let mut data = [0; 63];
                        data.copy_from_slice(&packet[1..]);
                        return (packet[0], data);
                    }
                    Err(RecvError::Lagged(_)) => continue,
                    Err(RecvError::Closed) => panic!("channel to packet receiver thread dropped"),
                }
            }
        }
    }
}
