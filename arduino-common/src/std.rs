extern crate std;
use core::{future::Future, ops::DerefMut, task::Poll, time::Duration};
use serialport::{SerialPort, TTYPort};
use std::io::{Read, Write};
use tokio::sync::Mutex;

use crate::prelude::*;
struct Reader<'a> {
    com: &'a mut dyn SerialPort,
}
impl<'a> Reader<'a> {
    fn new(com: &'a mut dyn SerialPort) -> Self {
        Self { com }
    }
}

impl<'a> Future for Reader<'a> {
    type Output = u8;

    fn poll(
        mut self: core::pin::Pin<&mut Self>,
        cx: &mut core::task::Context<'_>,
    ) -> core::task::Poll<Self::Output> {
        let mut buf = [0u8];
        if Read::read(self.com, &mut buf).is_ok() {
            Poll::Ready(buf[0])
        } else {
            cx.waker().wake_by_ref();
            Poll::Pending
        }
    }
}

impl AsyncSerial for TTYPort {
    async fn read(&mut self) -> u8 {
        Reader::new(self).await
    }

    async fn write(&mut self, buf: u8) {
        while self.write_all(&[buf]).is_err() {}
        self.flush().unwrap();
    }
}

///function used to sleep in std enviroments
impl Sleep for tokio::time::Sleep {
    fn await_us(us: u64) -> Self {
        //println!("wait");
        tokio::time::sleep(Duration::from_micros(us))
    }
}

impl<T> MutexTrait<T> for tokio::sync::Mutex<T> {
    fn new(t: T) -> Self {
        Self::new(t)
    }

    fn mut_lock(&self) -> impl Future<Output = impl DerefMut<Target = T>> {
        self.lock()
    }
}

pub type TokioMaster<Serial> = Master<Serial, tokio::time::Sleep, Mutex<InnerMaster<Serial, tokio::time::Sleep>>>;
