#[cfg(feature = "tokio")]
mod tokio;

#[cfg(feature = "embassy")]
mod embassy;

#[cfg(feature = "tokio")]
pub type Channel<T> = self::tokio::TokioMpsc<T>;
#[cfg(feature = "tokio")]
pub type Mutex<T> = self::tokio::TokioMutex<T>;
#[cfg(feature = "tokio")]
pub type TimeDriver = self::tokio::TokioTimeDriver;

#[cfg(feature = "embassy")]
pub type Channel<T> = self::embassy::EmbassyMpsc<T>;
#[cfg(feature = "embassy")]
pub type Mutex<T> = self::embassy::EmbassyMutex<T>;
#[cfg(feature = "embassy")]
pub type TimeDriver = self::embassy::EmbassyTimeDriver;
