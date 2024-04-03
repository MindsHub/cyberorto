#![no_std]

use core::{future::Future, mem};

use postcard::{from_bytes, to_slice};
use serde::{Deserialize, Serialize};
use serialmessage::{ParseState, SerMsg};

#[cfg(feature = "std")]
pub mod testable;

#[cfg(feature = "std")]
pub mod pc;

pub trait Serial {
    ///tries to read a single byte from Serial
    fn read(&mut self) -> Option<u8>;
    ///writes a single byte over Serial
    fn write(&mut self, buf: u8) -> bool;
}
pub trait AsyncSerial {
    ///tries to read a single byte from Serial
    fn read(&mut self) -> impl Future<Output = u8>;
    ///writes a single byte over Serial
    fn write(&mut self, buf: u8) -> impl Future<Output = ()>;
}

pub trait Timer {
    ///get ms_from_start
    fn ms_from_start(&self) -> u64;
}

/// -> message
/// <- Response

#[repr(u8)]
#[derive(Serialize, Deserialize, Debug)]
pub enum Message {
    /// variant to move motor
    ///
    Move {
        x: f32,
        y: f32,
        z: f32,
    },
    Debug([u8; 10]),
}

#[repr(u8)]
#[derive(Serialize, Deserialize, Debug)]
pub enum Response {
    WhoAreYou,
    Wait { ms: u64 },
}

/// parses the data into a common message
pub struct Comunication<S: Serial> {
    id: u8,
    serial: S,
    input_msg: SerMsg,
}
impl<S: Serial> Comunication<S> {
    pub fn new(serial: S) -> Self {
        Self {
            id: 0,
            serial,
            input_msg: SerMsg::new(),
        }
    }

    pub fn raw_send(&mut self, send_data: &[u8]) {
        let (msg, len) = SerMsg::create_msg_arr(send_data, 1).unwrap();
        for c in &msg[..len] {
            self.serial.write(*c);
        }
    }
    pub fn raw_read(&mut self) -> Option<&[u8]> {
        while let Some(c) = self.serial.read() {
            let (state, _len) = self.input_msg.parse_read_bytes(&[c]);
            match state {
                ParseState::DataReady => {
                    let t = self.input_msg.return_read_data();
                    let _id = self.input_msg.return_msg_id();
                    return Some(t);
                }
                _ => {}
            }
        }
        None
    }
    pub fn send_serialize<T: Serialize>(&mut self, m: T) {
        let mut buf = [0u8; mem::size_of::<Message>()];
        let used_buf = to_slice(&m, &mut buf).unwrap();
        self.raw_send(&used_buf);
    }
    pub fn try_read_deserialize<T: for<'a> Deserialize<'a>>(&mut self) -> Option<(u8, T)> {
        while let Some(x) = self.serial.read() {
            let (state, _) = self.input_msg.parse_read_bytes(&[x]);
            match state {
                ParseState::DataReady => {
                    let data = self.input_msg.return_read_data();
                    let m = from_bytes(data).ok()?;
                    let id = self.input_msg.return_msg_id();
                    return Some((id, m));
                }
                _ => {}
            }
        }
        None
    }
}
#[test]
fn test_out() {
    let m = Message::Move {
        x: 1.0,
        y: 2.0,
        z: 10.0,
    };
    let mut buf = [0u8; 32];
    let t = to_slice(&m, &mut buf).unwrap();
    panic!("{:?}", t);
}
#[cfg(test)]
mod test {

    use postcard::from_bytes;
    use serialmessage::SerMsg;

    use crate::Message;

    #[test]
    fn decompile() {
        let v = [0x7E, 0x01, 0xFF, 0x04, 0x01, 0xB0, 0xD5, 0x04, 0xD1, 0x81];
        let mut msg = SerMsg::new();
        let _ = msg.parse_read_bytes(&v);
        let data = msg.return_read_data();
        let msg = from_bytes::<Message>(data).unwrap();
        panic!("{:?}", msg);
    }
}
