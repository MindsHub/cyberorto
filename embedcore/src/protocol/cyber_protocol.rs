use serde::{Deserialize, Serialize};

#[repr(u8)]
#[non_exhaustive]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
/// In our comunication protocol we send this structure from master-> slave
pub enum Message {
    /// asking for information about the slave.
    /// EVERY DEVICE SHOULD RESPOND TO THAT
    WhoAreYou,
    /// variant to move motor
    MoveMotor { x: f32 },
    /// reset this motor
    ResetMotor,
    /// get various state info from the slave
    State,

    /// how is a previous blocking operation going?
    Poll,

    /// open water for duration ms. It should not be blocking and set a timeout
    Water { duration_ms: u64 },
    /// turn on lights for duration ms. It should not be blocking and set a timeout
    Lights { duration_ms: u64 },
    /// turn pump on for duration ms. It should not be blocking and set a timeout
    Pump { duration_ms: u64 },
    /// turn on plow for duration ms. It should not be blocking and set a timeout
    Plow { wait_ms: u64 },
    ///set status led
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
    async fn water(&mut self, ms: u64) -> Option<Response> {
        None
    }
    async fn lights(&mut self, ms: u64) -> Option<Response> {
        None
    }
    async fn pump(&mut self, ms: u64) -> Option<Response> {
        None
    }
    async fn plow(&mut self, ms: u64) -> Option<Response> {
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
    Iam { name: [u8; 10], version: u8 },

    /// you should wait for around ms
    Wait { ms: u64 },

    /// send debug message
    Debug([u8; 10]),

    /// TODO split fields from motors from fields from other sensors
    State { water: bool, lights: bool, pump: bool, plow: bool, led: bool, motor_pos: f32 },

    /// All ok
    Done,
}
