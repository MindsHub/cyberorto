
extern crate std;
use std::boxed::Box;
use serialport::SerialPort;

use crate::Serial;

impl Serial for Box<dyn SerialPort>{
    fn read(&mut self) -> Option<u8> {
        let mut buf = [0u8];
        let read = self.as_mut().read(&mut buf).ok()?;
        if read>0{
            Some(buf[0])
        }else{
            None
        }

    }

    fn write(&mut self, buf: u8)->bool {
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