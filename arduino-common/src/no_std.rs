use core::{
    cell::{RefCell, RefMut},
    future::Future,
    ops::DerefMut,
    pin::{pin, Pin},
    task::{Context, Poll},
};

use crate::prelude::*;

pub struct AwaitLock<'a, T: 'a> {
    lock: &'a RefCell<T>,
}
impl<'a, T: 'a> AwaitLock<'a, T> {
    fn new(r: &'a RefCell<T>) -> Self {
        Self { lock: r }
    }
}
impl<'a, T: 'a> Future for AwaitLock<'a, T> {
    type Output = RefMut<'a, T>;

    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        if let Ok(lock) = self.lock.try_borrow_mut() {
            Poll::Ready(lock)
        } else {
            Poll::Pending
        }
    }
}

pub struct SingleCoreMutex<T> {
    inner: RefCell<T>,
}
impl<T> MutexTrait<T> for SingleCoreMutex<T> {
    fn new(t: T) -> Self {
        Self {
            inner: RefCell::new(t),
        }
    }

    async fn mut_lock(&self) -> impl DerefMut<Target = T> {
        pin!(AwaitLock::new(&self.inner)).await
    }
}
