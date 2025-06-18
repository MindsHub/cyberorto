use super::{
    AsyncSerial,
    comunication::Comunication,
    cyber_protocol::{Message, MessagesHandler, Response},
};

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
                // TODO clarify what happens when a function returns None (at the moment no response is provided)
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
                    Message::State => {
                        if let Some(resp) = self.message_handler.state().await {
                            self.com.send(resp, id).await;
                        }
                    }
                    Message::Poll => {
                        if let Some(resp) = self.message_handler.poll().await {
                            self.com.send(resp, id).await;
                        }
                    }
                    Message::Water { cooldown_ms_or_off } => {
                        if let Some(resp) = self.message_handler.water(cooldown_ms_or_off).await {
                            self.com.send(resp, id).await;
                        }
                    }
                    Message::Lights { cooldown_ms_or_off } => {
                        if let Some(resp) = self.message_handler.lights(cooldown_ms_or_off).await {
                            self.com.send(resp, id).await;
                        }
                    }
                    Message::Pump { cooldown_ms_or_off } => {
                        if let Some(resp) = self.message_handler.pump(cooldown_ms_or_off).await {
                            self.com.send(resp, id).await;
                        }
                    }
                    Message::Plow { cooldown_ms_or_off } => {
                        if let Some(resp) = self.message_handler.plow(cooldown_ms_or_off).await {
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
