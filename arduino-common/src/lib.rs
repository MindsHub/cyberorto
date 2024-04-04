#![no_std]

use core::{future::Future, marker::PhantomData, pin::pin};

use futures::future::{select, Either};
use serde::{Deserialize, Serialize};
use serialmessage::{ParseState, SerMsg};

#[cfg(feature = "std")]
pub mod testable;

#[cfg(feature = "std")]
pub mod pc;

#[cfg(feature = "std")]
pub mod master;

#[cfg(feature = "std")]
pub mod tokio;

/*pub trait Serial {
    ///tries to read a single byte from Serial
    fn read(&mut self) -> Option<u8>;
    ///writes a single byte over Serial
    fn write(&mut self, buf: u8) -> bool;
}*/
pub trait AsyncSerial {
    ///tries to read a single byte from Serial
    fn read(&mut self) -> impl Future<Output = u8>;
    ///writes a single byte over Serial
    fn write(&mut self, buf: u8) -> impl Future<Output = ()>;
}

pub struct Comunication<Serial: AsyncSerial, Sleeper: Sleep> {
    ph: PhantomData<Sleeper>,
    timeout_us: u64,
    serial: Serial,
    input_buf: SerMsg,
    buf: [u8; 20],
}
pub trait Sleep: Future{
    fn await_us(us: u64)->Self;
}

impl<Serial: AsyncSerial, Sleeper: Sleep> Comunication<Serial, Sleeper> {
    pub fn new(serial: Serial, timeout_us: u64) -> Self {
        Self {
            ph: PhantomData,
            timeout_us,
            serial,
            input_buf: SerMsg::new(),
            buf: [0u8; 20],
        }
    }
    async fn try_read_byte(&mut self)->Option<u8>{
        match select(pin!(self.serial.read()), pin!(Sleeper::await_us(self.timeout_us))).await{
            Either::Left((b, _)) => Some(b),
            Either::Right(_) => None,
        }
    }
    async fn try_send_byte(&mut self, to_send: u8)->bool{
        match select(pin!(self.serial.write(to_send)), pin!(Sleeper::await_us(self.timeout_us))).await{
            Either::Left(_) => true,
            Either::Right(_) => {
                false
            },
        }
    }
    pub async fn try_read<Out: for<'a> Deserialize<'a>>(&mut self) -> Option<(u8, Out)> {
        while let Some(b) = self.try_read_byte().await {
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
        
        let Ok(msg) = postcard::to_slice(&to_send, &mut self.buf) else {
            return false;
        };
        let Some((buf, len)) = SerMsg::create_msg_arr(&msg, id) else {
            return false;
        };
        //println!("bytes wide {}", len);
        for b in &buf[0..len] {
            if !self.try_send_byte(*b).await {
                return false;
            }
        }
        true
    }
}

#[repr(u8)]
#[non_exhaustive]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Message {
    /// asking for information about the slave
    WhoAreYou,
    /// variant to move motor
    Move { x: f32, y: f32, z: f32 },
    Pool{id: u8},
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

    /// you should wait for around ms
    Wait {
        ms: u64,
    },
    ///send debug message
    Debug([u8; 10]),

    /// All ok
    Done,
}


pub struct Slave<Serial: AsyncSerial, Sleeper: Sleep> {
    com: Comunication<Serial, Sleeper>,
    /// what is my name?
    name: [u8; 10],
}
impl<Serial: AsyncSerial, Sleeper: Sleep> Slave<Serial, Sleeper> {

    pub fn new(serial: Serial, timeut_us: u64,  name: [u8; 10], ) -> Self {
        Self {
            com: Comunication::new(serial, timeut_us),
            name,
        }
    }
    /// let's run as Slave
    pub async fn run(&mut self) {
        loop {
            if let Some((id, message)) = self.com.try_read::<Message>().await {
                match message {
                    Message::WhoAreYou => {
                        self.com.send(
                            Response::Iam {
                                name: self.name,
                                version: 0,
                            },
                            id,
                        )
                        .await;
                    }
                    Message::Move { x: _, y: _, z: _ } => {
                        self.com.send(Response::Wait { ms: 0 }, id).await;
                    }
                    Message::Pool { id } => {
                        self.com.send(Response::Done, id).await;
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
    use tokio::time::Sleep;
    #[tokio::test]
    async fn test_slave() {
        let (master, slave) = Testable::new(0.0, 0.0);
        let mut master: Comunication<Testable, Sleep> = Comunication::new(master, 100);
        //let s = Comunication::new(slave);
        let name = b"ciao      ";
        let mut slave: Slave<Testable, Sleep> = Slave::new(slave, 100, name.clone());
        let q = tokio::spawn(async move {
            slave.run().await;
        });
        master.send(Message::WhoAreYou, 0).await;
        let (id, r) = master.try_read::<Response>().await.unwrap();
        assert_eq!(
            r,
            Response::Iam {
                name: name.clone(),
                version: 0
            }
        );
        assert_eq!(id, 0);
        q.abort();
    }

    #[tokio::test]
    async fn test_send_receive() {
        let (master, slave) = Testable::new(0.0, 0.0);
        let mut master: Comunication<Testable, Sleep> = Comunication::new(master, 100);
        let mut slave: Comunication<Testable, Sleep> = Comunication::new(slave, 100);
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
