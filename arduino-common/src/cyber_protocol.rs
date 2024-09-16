use serde::{Deserialize, Serialize};

use crate::prelude::*;

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

pub struct Slave<Serial: AsyncSerial, MA: MessagesHandler> {
    /// comunication interface, that permit to read/send messages
    com: Comunication<Serial>,
    /// what is my name?
    name: [u8; 10],
    /// struct used to handle all messages
    pub message_handler: MA,
}

impl<Serial: AsyncSerial, MA: MessagesHandler> Slave<Serial, MA> {
    /// init this struct, you should provide what serial you will use, and some other configs
    pub fn new(serial: Serial, timeout_us: u64, name: [u8; 10], message_handler: MA) -> Self {
        Self {
            com: Comunication::new(serial, timeout_us),
            name,
            message_handler,
        }
    }
    pub async fn run(&mut self) -> ! {
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
                    Message::MoveMotor { x } => {
                        if let Some(resp) = self.message_handler.move_motor(x).await {
                            self.com.send(resp, id).await;
                        }
                    }
                    Message::ResetMotor => {
                        if let Some(resp) = self.message_handler.reset_motor().await {
                            self.com.send(resp, id).await;
                        }
                    }
                    Message::Poll => {
                        if let Some(resp) = self.message_handler.poll().await {
                            self.com.send(resp, id).await;
                        }
                    }
                    Message::Water { duration_ms } => {
                        if let Some(resp) = self.message_handler.water(duration_ms).await {
                            self.com.send(resp, id).await;
                        }
                    }
                    Message::Lights { duration_ms } => {
                        if let Some(resp) = self.message_handler.lights(duration_ms).await {
                            self.com.send(resp, id).await;
                        }
                    }
                    Message::Pump { duration_ms } => {
                        if let Some(resp) = self.message_handler.pump(duration_ms).await {
                            self.com.send(resp, id).await;
                        }
                    }
                    Message::Plow { wait_ms } => {
                        if let Some(resp) = self.message_handler.plow(wait_ms).await {
                            self.com.send(resp, id).await;
                        }
                    }
                    Message::SetLed { led } => {
                        if let Some(resp) = self.message_handler.set_led(led).await {
                            self.com.send(resp, id).await;
                        }
                    }
                }
            }
        }
    }
}
