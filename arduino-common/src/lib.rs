#![no_std]
use core::fmt::Debug;
use core::{marker::PhantomData, time::Duration};

use prelude::*;
use serde::{Deserialize, Serialize};
pub mod motor;
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

pub mod cyber_protocol;

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
            //let id = $lock.as_mut().unwrap().id;

            while let Some((id_read, msg)) = $lock.as_mut().unwrap().try_read::<Response>().await {
                if id_read != $lock.as_mut().unwrap().id {
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

    pub async fn reset(&mut self) -> Result<(), ()> {
        todo!();
    }

    pub async fn home(&mut self) -> Result<(), ()> {
        todo!();
    }

    pub async fn retract(&mut self) -> Result<(), ()> {
        todo!();
    }

    pub async fn water(&mut self, water_state: Duration) -> Result<(), ()> {
        let m = Message::Water {
            duration_ms: water_state.as_millis() as u64,
        };
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
    pub async fn move_to(&self, pos: f32) -> Result<(), ()> {
        let m = Message::MoveMotor { x: pos };
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

    pub async fn lights(&mut self, duration: Duration) -> Result<(), ()> {
        todo!();
    }

    pub async fn pump(&mut self, pump_state: Duration) -> Result<(), ()> {
        todo!();
    }

    pub async fn plow(&self, duration: Duration) -> Result<(), ()> {
        let m = Message::Plow {
            wait_ms: duration.as_millis() as u64,
        };
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
    pub async fn set_led(&self, led: bool) -> Result<(), ()> {
        let m = Message::SetLed { led };
        let mut lock = Some(self.inner.mut_lock().await);
        blocking_send!(self, lock, m:
            Response::Done => {
                return Ok(());
            },
            _ => {}
        );
        Err(())
    }

    pub async fn who_are_you(&self) -> Result<([u8; 10], u8), ()> {
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
