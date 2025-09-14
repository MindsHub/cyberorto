use ch32_hal::{
    gpio::Output, mode, usart::{self, Uart, UartRx, UartTx}
};

use crate::protocol::{comunication::CommunicationError, AsyncSerial};

pub struct SerialWrapper<'a, T: usart::Instance> {
    pub rx: UartRx<'a, T, mode::Async>,
    pub tx: UartTx<'a, T, mode::Async>,
    status_pin: Option<Output<'a>>,
}
impl<'a, T: usart::Instance> SerialWrapper<'a, T> {
    pub fn new(u: Uart<'a, T, mode::Async>, status_pin: Option<Output<'a>>) -> Self {
        let (tx, rx) = u.split();
        Self { rx, tx, status_pin }
    }
}

impl<'a, T: usart::Instance> AsyncSerial for SerialWrapper<'a, T> {
    async fn read(&mut self) -> Result<u8, CommunicationError> {
        if let Some(pin) = &mut self.status_pin {
            pin.toggle();
        }
        let mut to_read = [0u8; 1];
        match self.rx.read(&mut to_read).await {
            Ok(()) => Ok(to_read[0]),
            Err(e) => Err(CommunicationError::ReadError { error: e, buffer_content: to_read[0] }),
        }
    }

    async fn write(&mut self, buf: u8) -> Result<(), CommunicationError> {
        match self.tx.write(&mut [buf]).await {
            Ok(()) => Ok(()),
            Err(e) => Err(CommunicationError::WriteError { error: e, buffer_content: buf }),
        }
    }
}
