#![no_std]
use core::{marker::PhantomData, time::Duration};
use core::fmt::Debug;
use prelude::*;
use serde::{Deserialize, Serialize};

#[cfg(feature = "std")]
pub mod testable;

#[cfg(feature = "std")]
pub mod std;

pub mod no_std;

pub mod traits;

pub mod prelude;
pub mod comunication;

#[repr(u8)]
#[non_exhaustive]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Message {
    /// asking for information about the slave
    WhoAreYou,
    /// variant to move motor
    Move {
        x: f32,
        y: f32,
        z: f32,
    },
    Reset {
        x: f32,
        y: f32,
        z: f32,
    },
    Retract {
        z: f32,
    },
    Pool {
        id: u8,
    },
    Water {
        water_state: Duration,
    },
    Lights {
        lights_state: Duration,
    },
    Pump {
        pump_state: Duration,
    },
    Plow {
        plow_state: Duration,
    },
}

#[repr(u8)]
#[non_exhaustive]
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub enum Response {
    /// response to WhoAreYou
    Iam { name: [u8; 10], version: u8 },

    /// you should wait for around ms
    Wait { ms: u64 },
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
    pub fn new(serial: Serial, timeut_us: u64, name: [u8; 10]) -> Self {
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
                        self.com
                            .send(
                                Response::Iam {
                                    name: self.name,
                                    version: 0,
                                },
                                id,
                            )
                            .await;
                    }
                    Message::Move { x: _, y: _, z: _ } => {
                        self.com.send(Response::Wait { ms: 1 }, id).await;
                    }
                    Message::Pool { id } => {
                        self.com.send(Response::Done, id).await;
                    }
                    Message::Reset { x, y, z } => todo!(),
                    Message::Retract { z } => todo!(),
                    Message::Water { water_state } => todo!(),
                    Message::Lights { lights_state } => todo!(),
                    Message::Pump { pump_state } => todo!(),
                    Message::Plow { plow_state } => todo!(),
                    _ => todo!()
                }
            }
        }
    }
}

pub struct InnerMaster<Serial: AsyncSerial, Sleeper: Sleep>{
    com: Comunication<Serial, Sleeper>,
    id: u8,
}

impl <Serial: AsyncSerial, Sleeper: Sleep> InnerMaster<Serial, Sleeper>{
    async fn send(&mut self, m: Message)->bool{
        self.id = self.id.wrapping_add(1);
        self.com.send(m, self.id).await
    }
    
    async fn try_read<Out: for<'a> Deserialize<'a>>(&mut self) -> Option<(u8, Out)> {
        self.com.try_read().await
    }
}



pub struct Master<Serial: AsyncSerial, Sleeper: Sleep, Mutex: MutexTrait<InnerMaster<Serial, Sleeper>>> {
    ph: PhantomData<Serial>,
    ph2: PhantomData<Sleeper>,
    inner: Mutex,
}

impl<Serial: AsyncSerial, Sleeper: Sleep, Mutex: MutexTrait<InnerMaster<Serial, Sleeper>>> Master<Serial, Sleeper, Mutex> {
    pub fn new(serial: Serial, timeout_us: u64) -> Self {
        Self {
            ph: PhantomData,
            ph2: PhantomData,
            inner: Mutex::new(InnerMaster{ com: Comunication::new(serial, timeout_us), id: 0}) ,
        }
    }
    pub async fn move_to(&self, x: f32, y: f32, z: f32) -> Result<(), ()> {
        let m = Message::Move { x, y, z };
        let mut lock = Some(self.inner.mut_lock().await);
        //retry only 10 times
        for _ in 0..10 {
            // send Move
            if !lock.as_mut().unwrap().send(m.clone()).await {
                continue;
            }
            let id = lock.as_mut().unwrap().id;

            while let Some((id_read, msg)) = lock.as_mut().unwrap().try_read::<Response>().await {
                if id_read != id {
                    continue;
                }
                match msg {
                    Response::Wait { ms } => {
                        lock.take();
                        Sleeper::await_us(ms * 1000).await;

                        lock = Some(self.inner.mut_lock().await);
                        if !lock.as_mut().unwrap().send(Message::Pool { id }).await {
                            continue;
                        }
                    }
                    Response::Done => {
                        return Ok(());
                    }
                    _ => {}
                }
            }
        }
        Err(())
    }


    pub async fn reset(&mut self, x: f32, y: f32, z: f32) -> Result<(), ()>{
        todo!();
    } 

    pub async fn retract(&mut self, z: f32) -> Result<(), ()>{
        todo!();
    } 

    pub async fn water(&mut self, water_state: Duration) -> Result<(), ()>{
        todo!();
    } 

    pub async fn lights(&mut self, lights_state: Duration) -> Result<(), ()>{
        todo!();
    } 

    pub async fn pump(&mut self, pump_state: Duration) -> Result<(), ()>{
        todo!();
    } 

    pub async fn plow(&mut self, plow_state: Duration) -> Result<(), ()>{
        todo!();
    } 

    pub async fn who_are_you(&mut self) -> Result<([u8; 10], u8), ()> {
        let mut lock = self.inner.mut_lock().await;

        for _ in 0..50 {
            if !lock.send(Message::WhoAreYou).await {
                continue;
            }
            let id = lock.id;

            while let Some((id_read, msg)) = lock.try_read::<Response>().await {
                if id_read != id {
                    continue;
                }
                if let Response::Iam { name, version } = msg {
                    //println!("resend {} ", i);
                    return Ok((name, version));
                }
            }
        }
        Err(())
    }
}

impl<Serial: AsyncSerial, Sleeper: Sleep, Mutex: MutexTrait<InnerMaster<Serial, Sleeper>>,> Debug for Master<Serial, Sleeper, Mutex>{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Master").finish()
    }
}