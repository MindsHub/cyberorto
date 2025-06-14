use ch32_hal::{
    mode,
    usart::{self, Uart, UartRx, UartTx},
};

use crate::protocol::AsyncSerial;

pub struct SerialWrapper<'a, T: usart::Instance> {
    rx: UartRx<'a, T, mode::Async>,
    tx: UartTx<'a, T, mode::Async>,
}
impl<'a, T: usart::Instance> SerialWrapper<'a, T> {
    pub fn new(u: Uart<'a, T, mode::Async>) -> Self {
        let (tx, rx) = u.split();
        Self { rx, tx }
    }
}

impl<'a, T: usart::Instance> AsyncSerial for SerialWrapper<'a, T> {
    async fn read(&mut self) -> u8 {
        let mut to_read = [0u8; 1];
        if let Ok(_) = self.rx.read(&mut to_read).await {
            to_read[0]
        } else {
            0
        }
    }

    async fn write(&mut self, buf: u8) {
        let _ = self.tx.write(&mut [buf]).await;
    }
}
