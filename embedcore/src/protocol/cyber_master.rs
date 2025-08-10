use core::marker::PhantomData;

use crate::{blocking_send, protocol::cyber::{DeviceIdentifier, ResponseState}, wait};
use core::fmt::Debug;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, mutex::Mutex};
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
        self.com.try_read().await.ok()
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

    pub async fn reset(&self) -> Result<(), ()> {
        todo!();
    }

    /// See [Message::MoveMotor]
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

    /// See [Message::Water]
    pub async fn water(&self, cooldown_ms: u64) -> Result<(), ()> {
        let m = Message::Water { cooldown_ms };
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

    /// See [Message::Lights]
    pub async fn lights(&self, _cooldown_ms: u64) -> Result<(), ()> {
        todo!();
    }

    /// See [Message::Pump]
    pub async fn pump(&self, _cooldown_ms: u64) -> Result<(), ()> {
        todo!();
    }

    /// See [Message::Plow]
    pub async fn plow(&self, cooldown_ms: u64) -> Result<(), ()> {
        let m = Message::Plow { cooldown_ms };
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

    pub async fn who_are_you(&self) -> Result<DeviceIdentifier, ()> {
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
                if let Response::Iam(device_identifier) = msg {
                    return Ok(device_identifier);
                }
            }
        }
        Err(())
    }

    pub async fn get_state(&self) -> Result<ResponseState, ()> {
        let m = Message::State;
        let mut lock = Some(self.inner.lock().await);
        blocking_send!(self, lock, m:
            Response::State(state) => {
                return Ok(state);
            },
            _ => {}
        );
        Err(())
    }
}

///debug implementation for Master
impl<Serial: AsyncSerial> Debug for Master<Serial> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Master").finish()
    }
}
