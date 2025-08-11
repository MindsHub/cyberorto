use serde::{Deserialize, Serialize};

#[repr(u8)]
#[non_exhaustive]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
/// In our comunication protocol we send this structure from master-> slave
pub enum Message {
    /// asking for information about the slave.
    /// EVERY DEVICE SHOULD RESPOND TO THAT
    WhoAreYou,
    /// emergency stop
    EmergencyStop,
    /// variant to move motor
    MoveMotor { x: f32 },
    /// reset this motor
    ResetMotor,
    /// get various state info from the slave
    State,

    /// how is a previous blocking operation going?
    Poll,

    /// Open or close water.
    /// To open water, provide a duration in milliseconds that acts as an automatic cooldown
    /// that avoids the water remaining open forever. To close water pass `0`.
    Water { cooldown_ms: u64 },
    /// Turn lights on or off.
    /// To turn lights on, provide a duration in milliseconds that acts as an automatic cooldown
    /// that avoids the lights remaining on forever. To turn off the lights pass `0`.
    Lights { cooldown_ms: u64 },
    /// Turn the pump on or off.
    /// To turn the pump on, provide a duration in milliseconds that acts as an automatic cooldown
    /// that avoids the pump remaining on forever. To turn off the pump pass `0`.
    Pump { cooldown_ms: u64 },
    /// Turn the plow on or off (only works if the plow tool is connected).
    /// To turn the plow on, provide a duration in milliseconds that acts as an automatic cooldown
    /// that avoids the plow remaining on forever. To turn off the plow pass `0`.
    Plow { cooldown_ms: u64 },
    /// Set status led on or off.
    SetLed { led: bool },
}

#[allow(unused_variables, async_fn_in_trait)]
pub trait MessagesHandler {
    async fn move_motor(&mut self, x: f32) -> Option<Response> {
        None
    }
    async fn reset_motor(&mut self) -> Option<Response> {
        None
    }
    async fn state(&mut self) -> Option<Response> {
        None
    }
    async fn poll(&mut self) -> Option<Response> {
        None
    }
    async fn water(&mut self, cooldown_ms: u64) -> Option<Response> {
        None
    }
    async fn lights(&mut self, cooldown_ms: u64) -> Option<Response> {
        None
    }
    async fn pump(&mut self, cooldown_ms: u64) -> Option<Response> {
        None
    }
    async fn plow(&mut self, cooldown_ms: u64) -> Option<Response> {
        None
    }
    async fn set_led(&mut self, state: bool) -> Option<Response> {
        None
    }
}

#[repr(u8)]
#[non_exhaustive]
#[derive(Serialize, Deserialize, Debug)]
/// In our comunication protocol we send this structure from slave-> master. Master should check if it is reasonable for the command that it has sent.
pub enum Response {
    /// response to WhoAreYou
    Iam(DeviceIdentifier),

    /// you should wait for around ms
    Wait { ms: u64 },

    /// send debug message
    Debug([u8; 10]),

    /// TODO split fields from motors from fields from other sensors
    State(ResponseState),

    /// All ok
    Done,

    /// Err
    Error ([u8; 10]),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DeviceIdentifier {
    pub name: [u8; 10],
    pub version: u8,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ResponseState {
    pub water: bool,
    pub lights: bool,
    pub pump: bool,
    pub plow: bool,
    pub led: bool,
    pub motor_pos: f32,
}
