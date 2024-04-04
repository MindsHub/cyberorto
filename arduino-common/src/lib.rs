#![no_std]

use core::{future::Future, marker::PhantomData};

use serde::{Deserialize, Serialize};
use serialmessage::{ParseState, SerMsg};

#[cfg(feature = "std")]
pub mod testable;

#[cfg(feature = "std")]
pub mod pc;

/*pub trait Serial {
    ///tries to read a single byte from Serial
    fn read(&mut self) -> Option<u8>;
    ///writes a single byte over Serial
    fn write(&mut self, buf: u8) -> bool;
}*/
pub trait AsyncSerial {
    ///tries to read a single byte from Serial
    fn read(&mut self) -> impl Future<Output = Option<u8>>;
    ///writes a single byte over Serial
    fn write(&mut self, buf: u8) -> impl Future<Output = bool>;
}

pub trait Timer {
    ///get ms_from_start
    fn ms_from_start(&self) -> u64;
}

pub struct Comunication<Serial: AsyncSerial> {
    serial: Serial,
    input_buf: SerMsg,
    buf: [u8; 20],
}
impl<Serial: AsyncSerial> Comunication<Serial> {
    pub fn new(serial: Serial) -> Self {
        Self {
            serial,
            input_buf: SerMsg::new(),
            buf: [0u8; 20],
        }
    }

    pub async fn try_read<Out: for<'a> Deserialize<'a>>(&mut self) -> Option<(u8, Out)> {
        while let Some(b) = self.serial.read().await {
            let (state, _) = self.input_buf.parse_read_bytes(&[b]);
            if let ParseState::DataReady = state {
                let data = self.input_buf.return_read_data();
                let id = self.input_buf.return_msg_id();
                return Some((id, postcard::from_bytes(data).ok()?));
            }
        }
        None
    }

    pub async fn send<Input: Serialize>(&mut self, to_send: Input, id: u8) -> bool {
        let Ok(msg) = postcard::to_slice(&to_send, &mut self.buf) else{ return false};
        let Some((buf, len)) = SerMsg::create_msg_arr(&msg, id) else{ return false};
        for b in &buf[0..len] {
            if !self.serial.write(*b).await {
                return false;
            }
        }
        true
    }
}   

#[repr(u8)]
#[non_exhaustive]
#[derive(Serialize, Deserialize, Debug)]
pub enum Message {
    /// asking for information about the slave
    WhoAreYou,
    /// variant to move motor
    Move { x: f32, y: f32, z: f32 },
}

#[repr(u8)]
#[non_exhaustive]
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub enum Response {
    /// response to WhoAreYou
    Iam {
        name: [u8; 10],
        version: u8,
    },
    Wait {
        ms: u64,
    },
    Debug([u8; 10]),
    Done,
}
pub struct Slave<Serial: AsyncSerial>{
    ph: PhantomData<Serial>,
    name: [u8; 10],
}
impl<Serial: AsyncSerial> Slave<Serial>{
    pub fn new(name: [u8; 10])->Self{
        Self { ph: PhantomData, name }
    }
    pub async fn run(&mut self, mut com: Comunication<Serial>){
        loop{
            if let Some((id, message)) = com.try_read::<Message>().await{
                match message{
                    Message::WhoAreYou => {
                        com.send(Response::Iam { name: self.name, version: 0 }, id).await;
                    },
                    Message::Move {  x: _, y: _, z: _ } => {
                        com.send(Response::Wait { ms: 1 }, id).await;
                    },
                }
            }
        }
    }
}


#[cfg(all(test, feature = "std"))]
mod test {



    use postcard::from_bytes;
    use serialmessage::SerMsg;

    use crate::{testable::Testable, Comunication, Message, Response, Slave};
    #[tokio::test]
    async fn test_slave(){
        let (master, slave) = Testable::new(0.0, 0.0);
        let mut master = Comunication::new(master);
        let s = Comunication::new(slave);
        let name = b"ciao      ";
        let mut slave = Slave::new(name.clone());
        let q = tokio::spawn(async move {
            
            slave.run(s).await;
            
        });
        master.send(Message::WhoAreYou, 0).await;        
        let (id, r) = master.try_read::<Response>().await.unwrap();
        assert_eq!( r, Response::Iam { name: name.clone(), version: 0 });
        assert_eq!(id, 0);
        q.abort();
    }
    #[tokio::test]
    async fn test_send_receive(){
        let (master, slave) = Testable::new(0.0, 0.0);
        let mut master = Comunication::new(master);
        let mut slave = Comunication::new(slave);
        master.send(Message::WhoAreYou, 0).await;
        slave.try_read::<Message>().await.unwrap();
    }
    #[test]
    fn decompile() {
        let v = [0x7E, 0x01, 0xFF, 0x04, 0x01, 0xB0, 0xD5, 0x04, 0xD1, 0x81];
        let mut msg = SerMsg::new();
        let _ = msg.parse_read_bytes(&v);
        let data = msg.return_read_data();
        let _msg = from_bytes::<Response>(data).unwrap();
        //panic!("{:?}", msg);
    }
}
