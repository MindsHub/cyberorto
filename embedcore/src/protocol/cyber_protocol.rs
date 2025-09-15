use core::fmt::Debug;

use serde::{Deserialize, Serialize};

#[repr(u8)]
#[non_exhaustive]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
/// In our comunication protocol we send this structure from the master to the slave.
/// Any message may reply with [Response::Unsupported] or [Response::Error] if something is wrong.
pub enum Message {
    /// Requests information about the slave. Normally replies with [Response::IAm].
    /// EVERY SLAVE DEVICE IS EXPECTED TO REPLY TO THIS MESSAGE.
    WhoAreYou,

    // messages only for a slave connected to a motor:

    /// Get state about the motor (useful for active controlling).
    /// Normally replies with [Response::MotorState].
    GetMotorState,
    /// Reset this motor. Normally replies with [Response::Ok].
    ResetMotor,
    /// Move the motor to the specified position in steps.
    /// Normally replies with [Response::Ok].
    MoveMotor { x: f32 },

    // messages only for the slave that handles peripherals:

    /// Get on/off state about the peripherals that this slave handles.
    /// Normally replies with [Response::PeripheralsState].
    GetPeripheralsState,
    /// Open or close water. Normally replies with [Response::Ok].
    /// To open water, provide a duration in milliseconds that acts as an automatic cooldown
    /// that avoids the water remaining open forever. To close water pass `0`.
    Water { cooldown_ms: u64 },
    /// Turn lights on or off. Normally replies with [Response::Ok].
    /// To turn lights on, provide a duration in milliseconds that acts as an automatic cooldown
    /// that avoids the lights remaining on forever. To turn off the lights pass `0`.
    Lights { cooldown_ms: u64 },
    /// Turn the pump on or off. Normally replies with [Response::Ok].
    /// To turn the pump on, provide a duration in milliseconds that acts as an automatic cooldown
    /// that avoids the pump remaining on forever. To turn off the pump pass `0`.
    Pump { cooldown_ms: u64 },
    /// Turn the plow on or off (only works if the plow tool is connected). Normally replies with
    /// [Response::Ok].
    /// To turn the plow on, provide a duration in milliseconds that acts as an automatic cooldown
    /// that avoids the plow remaining on forever. To turn off the plow pass `0`.
    Plow { cooldown_ms: u64 },
    /// Set status led on or off. Normally replies with [Response::Ok].
    SetLed { led: bool },
}

/// Note: there is no hook for [Message::WhoAreYou] here, as that's handled by the
/// [crate::protocol::cyber_slave::Slave] implementation directly (without passing through the
/// message handler.
#[allow(unused_variables, async_fn_in_trait)]
pub trait MessagesHandler {
    // functions only for a slave connected to a motor:

    async fn get_motor_state(&mut self) -> Response {
        Response::Unsupported
    }
    async fn move_motor(&mut self, x: f32) -> Response {
        Response::Unsupported
    }
    async fn reset_motor(&mut self) -> Response {
        Response::Unsupported
    }

    // functions only for the slave that handles peripherals:

    async fn get_peripherals_state(&mut self) -> Response {
        Response::Unsupported
    }
    async fn water(&mut self, cooldown_ms: u64) -> Response {
        Response::Unsupported
    }
    async fn lights(&mut self, cooldown_ms: u64) -> Response {
        Response::Unsupported
    }
    async fn pump(&mut self, cooldown_ms: u64) -> Response {
        Response::Unsupported
    }
    async fn plow(&mut self, cooldown_ms: u64) -> Response {
        Response::Unsupported
    }
    async fn set_led(&mut self, state: bool) -> Response {
        Response::Unsupported
    }
}

#[repr(u8)]
#[non_exhaustive]
#[derive(Serialize, Deserialize, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
/// In our comunication protocol we send this structure from the slave to the master.
/// The master should check if the obtained response is reasonable for the command that it has sent.
pub enum Response {

    // responses that could happen in response to any `Message`:

    /// This slave device cannot handle the received [Message].
    Unsupported,

    /// There was an error generating a response to the received [Message].
    Error([u8; 10]),


    // responses that could happen in response to only some `Message`s:

    /// The [Message] was an action to perform (e.g. turning water on/off, or setting the motor
    /// target position), and the action was performed correctly.
    Ok,

    /// Response to [Message::WhoAreYou].
    IAm(DeviceIdentifier),

    /// Response to [Message::GetMotorState].
    MotorState(MotorState),

    /// Response to [Message::GetPeripheralsState].
    PeripheralsState(PeripheralsState),
}

#[derive(Serialize, Deserialize, Clone)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct DeviceIdentifier {
    pub name: [u8; 10],
    pub version: u8,
}

impl Debug for DeviceIdentifier {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let mut f = f.debug_struct("DeviceIdentifier");
        match str::from_utf8(&self.name) {
            Ok(v) => f.field("name", &v),
            Err(_) => f.field("name", &self.name),
        };
        f.field("version", &self.version);
        f.finish()
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct PeripheralsState {
    pub water: bool,
    pub lights: bool,
    pub pump: bool,
    pub plow: bool,
    pub led: bool,
    pub battery_voltage: f32,
    pub water_scale: f32,
}

#[derive(Serialize, Deserialize, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct MotorState {
    pub motor_pos: f32,
    pub is_idle: bool,
    pub error: Option<[u8; 10]>,
}
