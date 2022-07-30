use cofit::Handler;
use core::{
    cell::UnsafeCell,
    future::Future,
    task::{Poll, Waker},
};
use futures::future::poll_fn;

/// Wrapper that immediately resolves, putting the message into a buffer for a background task to process
///
/// Note that this processing model may drop messages if a message arrives while the previous one has not been processed yet.
pub struct IndirectHandler<const MTU: usize, H: Handler<MTU>> {
    handler: H,
    waker: UnsafeCell<Option<Waker>>,
    message: UnsafeCell<Option<H::Message>>,
}

unsafe impl<const MTU: usize, H: Handler<MTU>> Send for IndirectHandler<MTU, H> {}
unsafe impl<const MTU: usize, H: Handler<MTU>> Sync for IndirectHandler<MTU, H> {}

impl<const MTU: usize, H: Handler<MTU>> IndirectHandler<MTU, H> {
    pub fn new(handler: H) -> Self {
        Self {
            handler,
            waker: UnsafeCell::new(None),
            message: UnsafeCell::new(None),
        }
    }

    /// Background task which needs to be polled for messages to be processed
    pub async fn task(&self) {
        loop {
            let message = self.next_message().await;
            self.handler.handle(message).await;
        }
    }

    #[must_use]
    fn next_message(&self) -> impl Future<Output = H::Message> + '_ {
        poll_fn(|cx| {
            critical_section::with(|_| unsafe {
                let waker = &mut *self.waker.get();
                let message = &mut *self.message.get();

                if let Some(message) = message.take() {
                    Poll::Ready(message)
                } else {
                    *waker = Some(cx.waker().clone());
                    Poll::Pending
                }
            })
        })
    }

    fn set_message(&self, new_message: H::Message) {
        critical_section::with(|_| unsafe {
            let waker = &mut *self.waker.get();
            let message = &mut *self.message.get();

            if message.is_some() {
                if let Some(w) = waker.take() {
                    w.wake();
                    *waker = None;
                } else {
                    // TODO Print a warning that we dropped a message
                }
            } else {
                *message = Some(new_message);

                if let Some(w) = waker.take() {
                    w.wake();
                    *waker = None;
                }
            }
        })
    }
}

impl<const MTU: usize, H: Handler<MTU>> Handler<MTU> for IndirectHandler<MTU, H> {
    type Message = H::Message;

    type RecvFut<'s> = impl Future<Output = ()> + 's
    where
        Self: 's;

    fn handle<'s>(&'s self, message: Self::Message) -> Self::RecvFut<'s> {
        async move { self.set_message(message) }
    }
}
