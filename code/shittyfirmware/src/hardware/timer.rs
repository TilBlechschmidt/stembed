use core::future::Future;
use core::ops::Add;
use embassy::time::{Duration, Instant, Timer};
use shittyruntime::time::*;

pub struct EmbassyTimer;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd)]
pub struct EmbassyDuration(Duration);

#[derive(Clone, Copy)]
pub struct EmbassyInstant(Instant);

impl TimeDriver for EmbassyTimer {
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
