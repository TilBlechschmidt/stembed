use core::future::Future;
use hidapi::HidDevice;
use shittyruntime::cofit::Transport;
use std::sync::{mpsc, Arc};
use tokio::sync::{
    broadcast::{self, error::RecvError},
    Mutex,
};

use self::wrapper::HidDeviceWrapper;

mod wrapper;

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
        let (hd_tx, hd_rx) = mpsc::channel::<[u8; 64]>();
        let (dh_tx, dh_rx) = broadcast::channel(256);

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

impl Transport<64> for UsbHidTransport {
    type TxFut<'t> = impl Future<Output = ()> + 't
    where
        Self: 't;

    type RxFut<'t> = impl Future<Output = [u8; 64]> + 't
    where
        Self: 't;

    fn send<'t>(&'t self, data: [u8; 64]) -> Self::TxFut<'t> {
        async move {
            self.tx
                .send(data)
                .expect("failed to forward packet to send thread")
        }
    }

    fn recv<'t>(&'t self) -> Self::RxFut<'t> {
        async move {
            loop {
                match self.rx.lock().await.recv().await {
                    Ok(packet) => return packet,
                    Err(RecvError::Lagged(_)) => continue,
                    Err(RecvError::Closed) => panic!("channel to packet receiver thread dropped"),
                }
            }
        }
    }
}
