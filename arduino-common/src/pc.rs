extern crate std;
use serialport::SerialPort;
use core::{future::Future, task::Poll};
use std::{boxed::Box, io::Read};

use crate::AsyncSerial;


struct Reader<'a>{
    com: &'a mut dyn SerialPort
}
 impl<'a> Reader<'a>{
    fn new(com: &'a mut dyn SerialPort)->Self{
        Self { com }
    }
 }
 impl<'a> Future for Reader<'a>{
    type Output=u8;
 
    fn poll(mut self: core::pin::Pin<&mut Self>, _cx: &mut core::task::Context<'_>) -> core::task::Poll<Self::Output> {
        let mut buf = [0u8];
        if Read::read( self.com, &mut buf).is_ok(){
            Poll::Ready(buf[0])
        }else{
            Poll::Pending
        }
    }
 }

impl AsyncSerial for Box<dyn SerialPort> {
    async fn read(&mut self) -> u8 {
        Reader::new( self.as_mut()).await
    }

    async fn write(&mut self, buf: u8) {
        while self.write_all(&[buf]).is_err(){}
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
