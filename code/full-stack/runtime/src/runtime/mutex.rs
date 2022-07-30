use core::{
    cell::UnsafeCell,
    ops::{Deref, DerefMut},
    sync::atomic::{AtomicBool, Ordering},
    task::{Poll, Waker},
};

/// Allows mutable access through a shared reference by locking the data
///
/// This Mutex can be poisoned by not invoking the guards `Drop` implementation (either by panicking or calling `mem::forget`).
///
/// This implementation only supports one concurrent waiting lock. However, if multiple futures use the same underlying Waker
/// (determined by the [`will_wake`](core::task::Waker::will_wake) fn), they may simultaneously await the Mutex.
///
/// It is explicitly not recommended to use this Mutex in a threaded scenario or anywhere outside of the same async task.
/// It has been purpose-built for the specific requirements of the runtime environment and thus may panic when used elsewhere.
pub struct Mutex<T> {
    value: UnsafeCell<T>,
    locked: AtomicBool,
    waker: UnsafeCell<Option<Waker>>,
}

/// Mutable handle to data protected by a [`Mutex`](self::Mutex)
pub struct MutexGuard<'m, T> {
    mutex: &'m Mutex<T>,
}

impl<T> Mutex<T> {
    pub fn new(value: T) -> Self {
        Self {
            value: UnsafeCell::new(value),
            locked: AtomicBool::new(false),
            waker: UnsafeCell::new(None),
        }
    }

    /// Attempts to gain access to the protected data, potentially yielding until a currently held lock is dropped
    pub async fn lock(&self) -> MutexGuard<'_, T> {
        futures::future::poll_fn(|cx| {
            let acquired = self
                .locked
                .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
                .is_ok();

            if acquired {
                Poll::Ready(MutexGuard { mutex: &self })
            } else {
                critical_section::with(|_| unsafe {
                    let waker = &mut *self.waker.get();
                    match waker {
                        Some(waker) if waker.will_wake(cx.waker()) => {}
                        Some(_) => panic!("waker overflow"),
                        None => *waker = Some(cx.waker().clone()),
                    }
                });
                Poll::Pending
            }
        })
        .await
    }

    /// Attempts to immediately gain access to the protected data, returning `None` if a lock is held elsewhere
    pub fn try_lock(&self) -> Option<MutexGuard<'_, T>> {
        let acquired = self
            .locked
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_ok();

        if acquired {
            Some(MutexGuard { mutex: &self })
        } else {
            None
        }
    }
}

impl<'m, T> Deref for MutexGuard<'m, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.mutex.value.get() }
    }
}

impl<'m, T> DerefMut for MutexGuard<'m, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.mutex.value.get() }
    }
}

impl<'m, T> Drop for MutexGuard<'m, T> {
    fn drop(&mut self) {
        self.mutex.locked.store(false, Ordering::Relaxed);

        critical_section::with(|_| unsafe {
            let waker = &mut *self.mutex.waker.get();
            if let Some(waker) = waker.take() {
                waker.wake();
            }
        })
    }
}
