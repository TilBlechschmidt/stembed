use core::future::Future;
use embassy::{
    blocking_mutex::raw::NoopRawMutex,
    mutex::MutexGuard,
    time::{Duration, Timer, Instant},
    util::Either,
};
use super::super::*;
use core::ops::Add;

pub struct EmbassyMutex<T>(embassy::mutex::Mutex<NoopRawMutex, T>);
pub struct EmbassyMpsc<T>(embassy::channel::mpmc::Channel<NoopRawMutex, T, 1>);
pub struct EmbassyMpscSender<'c, T>(&'c embassy::channel::mpmc::Channel<NoopRawMutex, T, 1>);
pub struct EmbassyMpscReceiver<'c, T>(&'c embassy::channel::mpmc::Channel<NoopRawMutex, T, 1>);
pub struct EmbassyTimeDriver;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd)]
pub struct EmbassyDuration(Duration);

#[derive(Clone, Copy)]
pub struct EmbassyInstant(Instant);

impl<T> Mutex for EmbassyMutex<T> {
    type Wrapped = T;

    type Guard<'m> = MutexGuard<'m, NoopRawMutex, T>
    where
        Self: 'm;

    type LockFut<'m> = impl Future<Output = Self::Guard<'m>> + 'm
    where
        Self: 'm;

    fn new(value: Self::Wrapped) -> Self {
        Self(embassy::mutex::Mutex::new(value))
    }

    fn lock<'m>(&'m self) -> Self::LockFut<'m> {
        self.0.lock()
    }

    fn try_lock<'m>(&'m self) -> Option<Self::Guard<'m>> {
        self.0.try_lock().ok()
    }
}

impl<T> Mpsc for EmbassyMpsc<T> {
    type Value = T;
    type Sender<'m> = EmbassyMpscSender<'m, Self::Value>
    where
        Self: 'm;
    type Receiver<'m> = EmbassyMpscReceiver<'m, Self::Value>
    where
        Self: 'm;

    fn new() -> Self {
        Self(embassy::channel::mpmc::Channel::new())
    }

    fn sender<'m>(&'m self) -> Self::Sender<'m> {
        EmbassyMpscSender(&self.0)
    }

    fn receiver<'m>(&'m self) -> Self::Receiver<'m> {
        EmbassyMpscReceiver(&self.0)
    }
}

impl<'c, T> MpscSender<T> for EmbassyMpscSender<'c, T> {
    type SendFut<'f> = impl Future<Output = ()> + 'f where Self: 'f;

    fn send<'f>(&'f self, value: T) -> Self::SendFut<'f> {
        self.0.send(value)
    }
}

impl<'c, T> MpscReceiver<T> for EmbassyMpscReceiver<'c, T> where T: 'c {
    type RecvFut<'f> = impl Future<Output = Option<T>> + 'f
    where
        Self: 'f;

    fn try_recv(&mut self) -> Option<T> {
        self.0.try_recv().ok()
    }

    fn recv_timeout(&mut self, timeout_ms: u32) -> Self::RecvFut<'_> {
        async move {
            let recv_fut = self.0.recv();
            let timeout_fut = Timer::after(Duration::from_millis(timeout_ms as u64));
            let result = embassy::util::select(recv_fut, timeout_fut);

            match result.await {
                Either::First(value) => Some(value),
                Either::Second(_) => None,
            }
        }
    }
}

impl TimeDriver for EmbassyTimeDriver {
    type Duration = EmbassyDuration;
    type Instant = EmbassyInstant;
    type TimerFut = impl Future<Output = ()>;

    fn now(&self) -> Self::Instant {
        EmbassyInstant(Instant::now())
    }

    fn wait_until(&self, instant: Self::Instant) -> Self::TimerFut {
        Timer::at(instant.0)
    }
}

impl DurationDriver for EmbassyDuration {}

impl InstantDriver for EmbassyInstant {
    type Duration = EmbassyDuration;

    fn elapsed(&self) -> Self::Duration {
        EmbassyDuration(self.0.elapsed())
    }
}

impl Add<EmbassyDuration> for EmbassyInstant {
    type Output = EmbassyInstant;

    fn add(self, rhs: EmbassyDuration) -> Self::Output {
        EmbassyInstant(self.0 + rhs.0)
    }
}

impl From<Duration> for EmbassyDuration {
    fn from(duration: Duration) -> Self {
        Self(duration)
    }
}
