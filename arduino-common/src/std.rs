extern crate std;
use core::{future::Future, ops::DerefMut, task::Poll};
use serialport::SerialPort;
use tokio::sync::Mutex;
use std::{boxed::Box, io::Read, time::Instant};

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

impl AsyncSerial for Box<dyn SerialPort> {
    async fn read(&mut self) -> u8 {
        Reader::new(self.as_mut()).await
    }

    async fn write(&mut self, buf: u8) {
        while self.write_all(&[buf]).is_err() {}
    }
}

///function used to sleep in std enviroments
pub struct StdSleeper{
    instant: Instant,
    us_to_wait: u128,
}

impl Future for StdSleeper{
    type Output=();

    fn poll(self: core::pin::Pin<&mut Self>, cx: &mut core::task::Context<'_>) -> Poll<Self::Output> {
        //println!("wait_pool");
        if self.instant.elapsed().as_micros()>self.us_to_wait{
            Poll::Ready(())
        }else{
            cx.waker().wake_by_ref();
            Poll::Pending
            
        }

    }
}
impl Sleep for StdSleeper{
    fn await_us(us: u64) -> Self {
        //println!("wait");
        Self { instant: Instant::now(), us_to_wait: us as u128 }
    }
}

impl<T> MutexTrait<T> for tokio::sync::Mutex<T>{
    fn new(t: T)->Self {
        Self::new(t)
    }

    fn mut_lock(& self)->impl Future<Output= impl DerefMut<Target =  T>> {
        self.lock()
    }
}

pub type TokioMaster<Serial> = Master<Serial, StdSleeper, Mutex<InnerMaster<Serial, StdSleeper>>>;

