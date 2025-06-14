use core::marker::PhantomData;

use crate::{blocking_send, wait};
use core::fmt::Debug;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, mutex::Mutex};
use embassy_time::Duration;
use serde::Deserialize;

use super::{
    AsyncSerial,
    comunication::Comunication,
    cyber_protocol::{Message, Response},
};

// this inner struct is behind a mutex. It should be possible to have multiple read-only references to the master struct and be able to send/read messages.
pub struct InnerMaster<Serial: AsyncSerial> {
    ///Comunication wrapper
    com: Comunication<Serial>,
    ///Last sent message id, before sending it get's increased by one until overflow appens, and then restarts from 0.
    id: u8,
}

impl<Serial: AsyncSerial> InnerMaster<Serial> {
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

pub struct Master<Serial: AsyncSerial> {
    /// first phantom data, nothing important
    ph: PhantomData<Serial>,
    /// Mutex for InnerMaster. It should get Locked when sending a message, when reading a response, and unlocked for everything else.
    inner: Mutex<CriticalSectionRawMutex, InnerMaster<Serial>>,
    /// how many times should a message be resent? Bigger numbers means better comunication but possibly slower.
    resend_times: u8,
}

impl<Serial: AsyncSerial> Master<Serial> {
    /// init a new Mutex
    pub fn new(serial: Serial, timeout_us: u64, resend_times: u8) -> Self {
        Self {
            ph: PhantomData,
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
            duration_ms: water_state.as_millis(),
        };
        let mut lock = Some(self.inner.lock().await);
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
        let mut lock = Some(self.inner.lock().await);
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

    pub async fn lights(&mut self, _duration: Duration) -> Result<(), ()> {
        todo!();
    }

    pub async fn pump(&mut self, _pump_state: Duration) -> Result<(), ()> {
        todo!();
    }

    pub async fn plow(&self, duration: Duration) -> Result<(), ()> {
        let m = Message::Plow {
            wait_ms: duration.as_millis(),
        };
        let mut lock = Some(self.inner.lock().await);
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
        let mut lock = Some(self.inner.lock().await);
        blocking_send!(self, lock, m:
            Response::Done => {
                return Ok(());
            },
            _ => {}
        );
        Err(())
    }

    pub async fn who_are_you(&self) -> Result<([u8; 10], u8), ()> {
        let mut lock = self.inner.lock().await;

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
                    return Ok((name, version));
                }
            }
        }
        Err(())
    }
}

///debug implementation for Master
impl<Serial: AsyncSerial> Debug for Master<Serial> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Master").finish()
    }
}
