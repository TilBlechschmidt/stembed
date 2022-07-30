//! Implementations of time & sync firmware traits for the tokio runtime on desktop

use super::super::{DurationDriver, InstantDriver, Mpsc, MpscReceiver, MpscSender, TimeDriver};
use core::future::Future;
use futures::future::Either;
use std::{ops::Add, pin::Pin};
use tokio::time::{sleep, sleep_until, Duration, Instant, Sleep};

pub struct TokioTimeDriver;

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub struct TokioDuration(Duration);

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub struct TokioInstant(Instant);

pub struct TokioMutex<T>(tokio::sync::Mutex<T>);
pub struct TokioMpsc<T> {
    tx: tokio::sync::broadcast::Sender<T>,
    #[allow(dead_code)]
    // This field has to be retained so tokio is happy, without it the channel will be closed once this last receiver is dropped
    rx: tokio::sync::broadcast::Receiver<T>,
}
pub struct TokioMpscSender<T>(tokio::sync::broadcast::Sender<T>);
pub struct TokioMpscReceiver<T>(tokio::sync::broadcast::Receiver<T>);

impl TimeDriver for TokioTimeDriver {
    type Duration = TokioDuration;
    type Instant = TokioInstant;
    type TimerFut = Pin<Box<Sleep>>;

    fn now(&self) -> Self::Instant {
        TokioInstant(Instant::now())
    }

    fn wait_until(&self, instant: Self::Instant) -> Self::TimerFut {
        Box::pin(sleep_until(instant.0))
    }
}

impl Add<TokioDuration> for TokioInstant {
    type Output = Self;

    fn add(self, rhs: TokioDuration) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl InstantDriver for TokioInstant {
    type Duration = TokioDuration;

    fn elapsed(&self) -> Self::Duration {
        TokioDuration(self.0.elapsed())
    }
}

impl DurationDriver for TokioDuration {}

impl<T> crate::firmware::Mutex for TokioMutex<T> {
    type Wrapped = T;

    type Guard<'m> = tokio::sync::MutexGuard<'m, T>
    where
        Self: 'm;

    type LockFut<'m> = impl Future<Output = Self::Guard<'m>>
    where
        Self: 'm;

    fn new(value: Self::Wrapped) -> Self {
        TokioMutex(tokio::sync::Mutex::new(value))
    }

    fn lock<'m>(&'m self) -> Self::LockFut<'m> {
        self.0.lock()
    }

    fn try_lock<'m>(&'m self) -> Option<Self::Guard<'m>> {
        self.0.try_lock().ok()
    }
}

impl<T> Mpsc for TokioMpsc<T>
where
    T: core::fmt::Debug + Clone,
{
    type Value = T;
    type Sender<'m> = TokioMpscSender<T> where T: 'm;
    type Receiver<'m> = TokioMpscReceiver<T> where T: 'm;

    fn new() -> Self {
        let (tx, rx) = tokio::sync::broadcast::channel(16);
        Self { tx, rx }
    }

    fn sender<'m>(&'m self) -> Self::Sender<'m> {
        TokioMpscSender(self.tx.clone())
    }

    fn receiver<'m>(&'m self) -> Self::Receiver<'m> {
        TokioMpscReceiver(self.tx.subscribe())
    }
}

impl<T> MpscSender<T> for TokioMpscSender<T>
where
    T: core::fmt::Debug,
{
    type SendFut<'f> = impl Future<Output = ()> + 'f where Self: 'f;

    fn send<'f>(&'f self, value: T) -> Self::SendFut<'f> {
        async move {
            self.0
                .send(value)
                .expect("failed to send message on tokio mpsc");
        }
    }
}

impl<T: Clone> MpscReceiver<T> for TokioMpscReceiver<T> {
    type RecvFut<'f> = impl Future<Output = Option<T>> + 'f where Self: 'f;

    fn try_recv(&mut self) -> Option<T> {
        self.0.try_recv().ok()
    }

    fn recv_timeout(&mut self, timeout_ms: u32) -> Self::RecvFut<'_> {
        async move {
            let recv_fut = self.0.recv();
            let timeout_fut = sleep(Duration::from_millis(timeout_ms as u64));
            let result = futures::future::select(Box::pin(recv_fut), Box::pin(timeout_fut));

            match result.await {
                Either::Left((value, _)) => value.ok(),
                Either::Right(_) => None,
            }
        }
    }
}
