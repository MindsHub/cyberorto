#![no_std]
use core::fmt::Debug;
use core::{marker::PhantomData, time::Duration};
use prelude::*;
use serde::{Deserialize, Serialize};

/// Implementation used while testing. It is behind "std" flag
#[cfg(feature = "std")]
pub mod testable;

/// common std implementations. It is behind "std" flag
#[cfg(feature = "std")]
pub mod std;

/// default, no std implementation. Not all the implementation can be used in a std context
pub mod no_std;

/// common traits, they are used to abstract all dependencies from hardware
pub mod traits;

/// Comunication wrapper. Is used to serialize/deserialize and send/read messages, but it doens't have any clue on what that packets contain
pub mod comunication;
/// comodity import. Just type use arduino-common::prelude::*; and you are ready to go
pub mod prelude;

#[repr(u8)]
#[non_exhaustive]
#[derive(Serialize, Deserialize, Debug, Clone)]
/// In our comunication protocol we send this structure from master-> slave
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
    Poll,
    Water {
        wait_ms: u64,
    },
    Lights {
        lights_state: Duration,
    },
    Pump {
        pump_state: Duration,
    },
    Plow {
        wait_ms: u64,
    },
    SetLed{
        led: bool,
    }
}

#[repr(u8)]
#[non_exhaustive]
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
/// In our comunication protocol we send this structure from slave-> master. Master should check if it is reasonable for the command that it has sent.
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

pub enum Command{
    Moving,
}

/// Robot state, it contains all relevant informations
pub struct BotState {
    pub obj_pos: Option<(f32, f32, f32)>,
    pub cur_pos: (f32, f32, f32),
    pub command: Option<Command>,
    pub led: bool,
}
impl BotState{
    pub fn new()->Self{
        BotState { obj_pos: None, cur_pos: (0.0, 0.0, 0.0), command: None, led: false}
    }
}

/// This is a slave builder. when we call run we get a never returning Futures to be polled.
pub struct SlaveBot<'a, Serial: AsyncSerial, Sleeper: Sleep, Mutex: MutexTrait<BotState>> {
    /// comunication interface, that permit to read/send messages
    com: Comunication<Serial, Sleeper>,
    /// what is my name?
    name: [u8; 10],
    /// inner state
    state: &'a Mutex,
}

impl<'a, Serial: AsyncSerial, Sleeper: Sleep, Mutex: MutexTrait<BotState>>
    SlaveBot<'a, Serial, Sleeper, Mutex>
{
    /// init this struct, you should provide what serial you will use, and some other configs
    pub fn new(serial: Serial, timeout_us: u64, name: [u8; 10], state: &'a Mutex) -> Self {
        Self {
            com: Comunication::new(serial, timeout_us),
            name,
            state,
        }
    }
    /// let's run as Slave. It should never returns
    pub async fn run(&mut self) -> ! {
        loop {
            if let Some((id, message)) = self.com.try_read::<Message>().await {
                let mut lock = self.state.mut_lock().await;
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
                    Message::Move { x, y, z } => {
                        lock.obj_pos = Some((x, y, z));
                        self.com.send(Response::Wait { ms: 100 }, id).await;
                    }
                    Message::Poll => {
                        if let Some(_) = &lock.command{
                            self.com.send(Response::Wait { ms: 100 }, id).await;
                        }else{
                            self.com.send(Response::Done, id).await;
                        }
                    }
                    Message::SetLed { led } => {
                        lock.led=led;
                        self.com.send(Response::Done, id).await;
                    },
                    _ => {},
                }
            }
        }
    }
}

///struct used inside Master. It wraps some Comunication methods. Watch Master Documentation to understand why it's important.
pub struct InnerMaster<Serial: AsyncSerial, Sleeper: Sleep> {
    ///Comunication wrapper
    com: Comunication<Serial, Sleeper>,
    ///Last sent message id, before sending it get's increased by one until overflow appens, and then restarts from 0.
    id: u8,
}

impl<Serial: AsyncSerial, Sleeper: Sleep> InnerMaster<Serial, Sleeper> {
    /// increments id by one, and then sends a message
    async fn send(&mut self, m: Message) -> bool {
        self.id = self.id.wrapping_add(1);
        self.com.send(m, self.id).await
    }

    ///tries to read a message
    async fn try_read<Out: for<'a> Deserialize<'a>>(&mut self) -> Option<(u8, Out)> {
        self.com.try_read().await
    }
}

pub struct Master<
    Serial: AsyncSerial,
    Sleeper: Sleep,
    Mutex: MutexTrait<InnerMaster<Serial, Sleeper>>,
> {
    /// first phantom data, nothing important
    ph: PhantomData<Serial>,
    /// second phantom data, nothing important
    ph2: PhantomData<Sleeper>,
    /// Mutex for InnerMaster. It should get Locked when sending a message, when reading a response, and unlocked for everything else.
    inner: Mutex,
    /// how many times should a message be resent? Bigger numbers means better comunication but possibly slower.
    resend_times: u8,
}
// self, lock, message: pattern => block...
macro_rules! blocking_send {
    ($self:ident, $lock:ident, $m:ident : $($p:pat => $block:block),+) => {
        
        for _ in 0..$self.resend_times {
            // send Move
            if !$lock.as_mut().unwrap().send($m.clone()).await {
                continue;
            }
            let id = $lock.as_mut().unwrap().id;

            while let Some((id_read, msg)) = $lock.as_mut().unwrap().try_read::<Response>().await {
                if id_read != id {
                    continue;
                }
                match msg {
                    $(
                        $p => $block
                    ),*
                }
            }
        }
    };
}
/// wait(self, lock, ms)
macro_rules! wait {
    ($self:ident, $lock:ident, $ms:ident) => {
        $lock.take();
        Sleeper::await_us($ms * 1000).await;

        $lock = Some($self.inner.mut_lock().await);
        if !$lock.as_mut().unwrap().send(Message::Poll).await {
            continue;
        }
    };
}
impl<Serial: AsyncSerial, Sleeper: Sleep, Mutex: MutexTrait<InnerMaster<Serial, Sleeper>>>
    Master<Serial, Sleeper, Mutex>
{
    /// init a new Mutex
    pub fn new(serial: Serial, timeout_us: u64, resend_times: u8) -> Self {
        Self {
            ph: PhantomData,
            ph2: PhantomData,
            inner: Mutex::new(InnerMaster {
                com: Comunication::new(serial, timeout_us),
                id: 0,
            }),
            resend_times,
        }
    }

    /// Let's tell the bot to move to a particular x, y, z point
    pub async fn move_to(&self, x: f32, y: f32, z: f32) -> Result<(), ()> {
        let m = Message::Move { x, y, z };
        
        let mut lock = Some(self.inner.mut_lock().await);

        blocking_send!(self, lock, m:
            Response::Wait { ms } => {
                wait!(self, lock, ms);
            },
            Response::Done => {
                return Ok(());
            },
            _ => {}
        );
        Err(())
    }

    pub async fn reset(&mut self, x: f32, y: f32, z: f32) -> Result<(), ()> {
        todo!();
    }

    pub async fn retract(&mut self, z: f32) -> Result<(), ()> {
        todo!();
    }

    pub async fn water(&mut self, water_state: Duration) -> Result<(), ()> {
        let m = Message::Water { wait_ms: water_state.as_millis() as u64 };
        let mut lock = Some(self.inner.mut_lock().await);
        blocking_send!(self, lock, m: 
            Response::Wait { ms } => {
                wait!(self, lock, ms);
            }, 
            Response::Done => {
                return Ok(())
            },
            _ => {}
        );
        Err(())
    }

    pub async fn lights(&mut self, duration: Duration) -> Result<(), ()> {
        todo!();
    }

    pub async fn pump(&mut self, pump_state: Duration) -> Result<(), ()> {
        todo!();
    }

    pub async fn plow(&mut self, duration: Duration) -> Result<(), ()> {
        let m = Message::Plow { wait_ms: duration.as_millis() as u64 };
        let mut lock = Some(self.inner.mut_lock().await);
        blocking_send!(self, lock, m: 
            Response::Wait { ms } => {
                wait!(self, lock, ms);
            }, 
            Response::Done => {
                return Ok(())
            },
            _ => {}
        );
        todo!();
    }
    pub async fn set_led(& self, led: bool)->Result<(), ()>{
        let m = Message::SetLed { led};
        let mut lock = Some(self.inner.mut_lock().await);
        blocking_send!(self, lock, m: 
            Response::Done => {
                return Ok(());
            },
            _ => {}
        );
        Err(())
    }

    pub async fn who_are_you(&mut self) -> Result<([u8; 10], u8), ()> {
        let mut lock = self.inner.mut_lock().await;

        for _ in 0..self.resend_times {
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

///debug implementation for Master
impl<Serial: AsyncSerial, Sleeper: Sleep, Mutex: MutexTrait<InnerMaster<Serial, Sleeper>>> Debug
    for Master<Serial, Sleeper, Mutex>
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Master").finish()
    }
}
