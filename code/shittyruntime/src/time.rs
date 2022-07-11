use core::future::Future;
use core::ops::Add;

pub trait TimeDriver {
    type Duration: DurationDriver;
    type Instant: InstantDriver<Duration = Self::Duration>;
    type TimerFut: Future<Output = ()> + Unpin;

    fn now(&self) -> Self::Instant;
    fn wait_until(&self, instant: Self::Instant) -> Self::TimerFut;
}

pub trait InstantDriver: Add<Self::Duration, Output = Self> + Copy {
    type Duration: DurationDriver;

    fn elapsed(&self) -> Self::Duration;
}

pub trait DurationDriver: PartialOrd + Copy {}
