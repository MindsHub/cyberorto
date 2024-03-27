

#![no_std]

//
//

use serialmessage::{ParseState, SerMsg};

#[cfg(feature = "std")]
pub mod testable;

pub trait Serial {
    fn read(&mut self) -> Option<u8>;
    //fn advance_buffer(&mut self, to_remove: usize);
    fn write(&mut self, buf: u8)->bool;
}

#[repr(u8)]
enum Message{
    Send=0,
    Ack=1,
}



pub struct Comunication<S: Serial>{
    serial: S,
    input_msg: SerMsg,
}
impl<S: Serial> Comunication<S>{
    pub fn new(serial: S)->Self{
        Self { serial , input_msg: SerMsg::new()}
    }

    pub fn send(&mut self, send_data: &[u8]){
        let (msg, len) = SerMsg::create_msg_arr(send_data, 1).unwrap();
        for c in &msg[..len]{
            self.serial.write(*c);
        }
    }
    pub fn read(&mut self)->Option<&[u8]>{
        while let Some(c) = self.serial.read(){
            let (state, _len) =self.input_msg.parse_read_bytes(&[c]);
            match state{
                ParseState::DataReady => {
                    let t = self.input_msg.return_read_data();
                    let _id = self.input_msg.return_msg_id();
                    return Some(t)
                },
                _ => {},
            }
        }
        None
    }
}

