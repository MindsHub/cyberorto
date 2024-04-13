use core::{future::Future, ops::DerefMut};

/// serial abstraction. It's considered infallible
pub trait AsyncSerial {
    ///tries to read a single byte from Serial
    fn read(&mut self) -> impl Future<Output = u8>;
    ///writes a single byte over Serial
    fn write(&mut self, buf: u8) -> impl Future<Output = ()>;
}
/// trait used to abstract a sleeper (Await some us and go on)
pub trait Sleep: Future {
    /// returns a struct to await
    fn await_us(us: u64) -> Self;
}

/// we need dynamic mutable access in order to use serial even if we are waiting for a big message.
///
/// This trait is needed to abstract from the hardware implementation of the mutex
pub trait MutexTrait<T> {
    fn new(t: T) -> Self;
    fn mut_lock(&self) -> impl Future<Output = impl DerefMut<Target = T>>;
}
