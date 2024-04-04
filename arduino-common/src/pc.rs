extern crate std;
use serialport::SerialPort;
use std::boxed::Box;

use crate::AsyncSerial;

impl AsyncSerial for Box<dyn SerialPort> {
    async fn read(&mut self) -> Option<u8> {
        let mut buf = [0u8];
        let read = self.as_mut().read(&mut buf).ok()?;
        if read > 0 {
            Some(buf[0])
        } else {
            None
        }
    }

    async fn write(&mut self, buf: u8) -> bool {
        self.write_all(&[buf]).is_ok()
    }
}
/*impl Serial for Box<dyn SerialPort>{
    fn read(&mut self) -> Option<u8> {
        todo!()
    }

    fn write(&mut self, buf: u8)->bool {
        todo!()
    }
}*/
