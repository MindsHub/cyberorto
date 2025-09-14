use core::marker::PhantomData;

use crate::{protocol::cyber::{DeviceIdentifier, MotorState, PeripheralsState}};
use core::fmt::Debug;
use defmt_or_log::{debug, trace};
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
        trace!("InnerMaster: sending message {:?} with id {}", m, self.id);
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
    /// how much time should we wait for a message, before trying to resend it?
    #[cfg(feature = "std")]
    timeout: core::time::Duration,
}

macro_rules! match_response {
    ($msg:expr, $($p:pat => $e:expr,)+) => {{
        let msg = $msg;
        match $msg {
            $crate::protocol::cyber::Response::Unsupported => {
                defmt_or_log::error!("match_response!: received response Unsupported");
                Err(())
            },
            $crate::protocol::cyber::Response::Error(e) => {
                defmt_or_log::error!("match_response!: received response Error({:?})", e);
                Err(())
            }
            $(
                $p => $e,
            ),*
            _ => {
                defmt_or_log::error!("match_response!: received unexpected response {:?}", msg);
                Err(())
            }
        }
    }};
}

impl<Serial: AsyncSerial> Master<Serial> {
    /// init a new Mutex
    pub fn new(
        serial: Serial,
        #[cfg(feature = "std")]
        timeout: core::time::Duration,
        resend_times: u8
    ) -> Self {
        Self {
            ph: PhantomData,
            inner: Mutex::new(InnerMaster {
                com: Comunication::new(serial),
                id: 0,
            }),
            resend_times,
        #[cfg(feature = "std")]
            timeout,
        }
    }

    #[cfg(feature = "std")]
    async fn send_message(&self, message: Message) -> Result<Response, ()> {
        let mut result: Result<Result<_, ()>, _> = Ok(Err(()));
        let mut lock = self.inner.lock().await;
        for _ in 0..self.resend_times {
            let future = async {
                defmt_or_log::debug!("send_message: sending {:?}", message);
                if !lock.send(message.clone()).await {
                    return Err(());
                }
                defmt_or_log::debug!("send_message: sent {:?}", message);

                while let Some((id_read, msg)) = lock.try_read::<Response>().await {
                    if id_read != lock.id {
                        continue;
                    }

                    return Ok(msg)
                }

                Err(())
            };
            result = tokio::time::timeout(self.timeout, future).await;

            if let Ok(r) = result {
                if r.is_ok() {
                    defmt_or_log::debug!("send_message: received {:?}", r);
                    return r;
                } else {
                    result = Ok(r);
                }
            }
            defmt_or_log::debug!("send_message: timeout");
        }

        match result {
            Ok(result) => result,
            Err(_) => Err(()),
        }
    }

    // Useless implementation, we don't use Master on embedded (i.e. no-std) anyway.
    #[cfg(not(feature = "std"))]
    async fn send_message(&self, message: Message) -> Result<Response, ()> {
        let mut lock = self.inner.lock().await;
        for _ in 0..self.resend_times {
            defmt_or_log::debug!("send_message: sending {:?}", message);
            if !lock.send(message.clone()).await {
                continue;
            }
            defmt_or_log::debug!("send_message: sent {:?}", message);

            // Replace the "while let" used in the implementation above with another for loop,
            // since we don't have timeouts here.
            // TODO if we end up having Master on embedded (i.e. no-std), properly implement async
            // timeouts.
            for _ in 0..self.resend_times {
                if let Some((id_read, msg)) = lock.try_read::<Response>().await {
                    if id_read != lock.id {
                        continue;
                    }

                    defmt_or_log::debug!("send_message: received {:?}", msg);
                    return Ok(msg)
                }
            }
        }

        Err(())
    }

    /// See [Message::WhoAreYou].
    pub async fn who_are_you(&self) -> Result<DeviceIdentifier, ()> {
        debug!("who_are_you(): called");
        match_response!(
            self.send_message(Message::WhoAreYou).await?,
            Response::IAm(device_identifier) => Ok(device_identifier),
        )
    }

    /// See [Message::GetMotorState].
    pub async fn get_motor_state(&self) -> Result<MotorState, ()> {
        match_response!(
            self.send_message(Message::GetMotorState).await?,
            Response::MotorState(motor_state) => Ok(motor_state),
        )
    }

    /// See [Message::ResetMotor].
    pub async fn reset_motor(&self) -> Result<(), ()> {
        match_response!(
            self.send_message(Message::ResetMotor).await?,
            Response::Ok => Ok(()),
        )
    }

    /// See [Message::MoveMotor].
    pub async fn move_motor(&self, pos: f32) -> Result<(), ()> {
        debug!("Move to called with pos = {}", pos);
        match_response!(
            self.send_message(Message::MoveMotor { x: pos }).await?,
            Response::Ok => Ok(()),
        )
    }

    /// See [Message::GetPeripheralsState].
    pub async fn get_peripherals_state(&self) -> Result<PeripheralsState, ()> {
        match_response!(
            self.send_message(Message::GetPeripheralsState).await?,
            Response::PeripheralsState(peripherals_state) => Ok(peripherals_state),
        )
    }

    /// See [Message::Water].
    pub async fn water(&self, cooldown_ms: u64) -> Result<(), ()> {
        match_response!(
            self.send_message(Message::Water { cooldown_ms }).await?,
            Response::Ok => Ok(()),
        )
    }

    /// See [Message::Lights].
    pub async fn lights(&self, cooldown_ms: u64) -> Result<(), ()> {
        match_response!(
            self.send_message(Message::Lights { cooldown_ms }).await?,
            Response::Ok => Ok(()),
        )
    }

    /// See [Message::Pump].
    pub async fn pump(&self, cooldown_ms: u64) -> Result<(), ()> {
        match_response!(
            self.send_message(Message::Pump { cooldown_ms }).await?,
            Response::Ok => Ok(()),
        )
    }

    /// See [Message::Plow].
    pub async fn plow(&self, cooldown_ms: u64) -> Result<(), ()> {
        match_response!(
            self.send_message(Message::Plow { cooldown_ms }).await?,
            Response::Ok => Ok(()),
        )
    }

    /// See [Message::SetLed].
    pub async fn set_led(&self, led: bool) -> Result<(), ()> {
        match_response!(
            self.send_message(Message::SetLed { led }).await?,
            Response::Ok => Ok(()),
        )
    }
}

///debug implementation for Master
impl<Serial: AsyncSerial> Debug for Master<Serial> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Master").finish()
    }
}
