extern crate std;
use core::{future::Future, ops::DerefMut};
use tokio::sync::Mutex;
use tokio_serial::SerialStream;

use crate::prelude::*;

impl AsyncSerial for SerialStream {
    async fn read(&mut self) -> u8 {
        let mut buf = [0u8];
        while tokio::io::AsyncReadExt::read(self, &mut buf).await.is_err() {}
        buf[0]
    }

    async fn write(&mut self, buf: u8) {
        while tokio::io::AsyncWriteExt::write(self, &[buf]).await.is_err() {}
        let _ = tokio::io::AsyncWriteExt::flush(self).await; // ignore the result
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

pub type TokioMaster<Serial> = Master<Serial, Mutex<InnerMaster<Serial>>>;

#[tokio::test]
async fn rapid_test() {
    let t = embassy_time::Instant::now();
    embassy_time::Timer::after_millis(1000).await;
    let t = embassy_time::Instant::now();
    std::println!("{}", t);
}
