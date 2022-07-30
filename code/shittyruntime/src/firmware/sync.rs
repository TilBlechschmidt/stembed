use core::future::Future;
use core::ops::DerefMut;

pub trait Mutex {
    type Wrapped;

    type Guard<'m>: DerefMut<Target = Self::Wrapped> + 'm
    where
        Self: 'm;

    type LockFut<'m>: Future<Output = Self::Guard<'m>>
    where
        Self: 'm;

    fn new(value: Self::Wrapped) -> Self;

    #[must_use]
    fn lock<'m>(&'m self) -> Self::LockFut<'m>;
    fn try_lock<'m>(&'m self) -> Option<Self::Guard<'m>>;
}

pub trait Mpsc {
    type Value;
    type Sender<'m>: MpscSender<Self::Value> + 'm
    where
        Self: 'm;
    type Receiver<'m>: MpscReceiver<Self::Value> + 'm
    where
        Self: 'm;

    fn new() -> Self;
    fn sender<'m>(&'m self) -> Self::Sender<'m>;
    fn receiver<'m>(&'m self) -> Self::Receiver<'m>;

    fn split<'m>(&'m self) -> (Self::Sender<'m>, Self::Receiver<'m>) {
        (self.sender(), self.receiver())
    }
}

pub trait MpscSender<T> {
    type SendFut<'f>: Future<Output = ()> + 'f
    where
        Self: 'f;

    #[must_use]
    fn send<'f>(&'f self, value: T) -> Self::SendFut<'f>;
}

pub trait MpscReceiver<T> {
    type RecvFut<'f>: Future<Output = Option<T>> + 'f
    where
        Self: 'f;

    /// Pulls any immediately available value from the receiver
    fn try_recv(&mut self) -> Option<T>;

    /// Waits at most `timeout_ms` milliseconds for a new message
    #[must_use]
    fn recv_timeout(&mut self, timeout_ms: u32) -> Self::RecvFut<'_>;

    /// Removes and discards any immediately available messages, returning the amount
    fn clear(&mut self) -> usize {
        let mut count = 0;
        while let Some(_) = self.try_recv() {
            count += 1;
        }
        count
    }
}
