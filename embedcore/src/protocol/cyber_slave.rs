use crate::protocol::cyber::DeviceIdentifier;

use super::{
    AsyncSerial,
    communication::Communication,
    cyber_protocol::{Message, MessagesHandler, Response},
};

pub struct Slave<Serial: AsyncSerial, MA: MessagesHandler> {
    /// communication interface, that permit to read/send messages
    pub com: Communication<Serial>,
    /// what is my name?
    device_identifier: DeviceIdentifier,
    /// struct used to handle all messages
    pub message_handler: MA,
}

impl<Serial: AsyncSerial, MA: MessagesHandler> Slave<Serial, MA> {
    /// init this struct, you should provide what serial you will use, and some other configs
    pub fn new(serial: Serial, name: [u8; 10], message_handler: MA) -> Self {
        Self {
            com: Communication::new(serial),
            device_identifier: DeviceIdentifier { name, version: 0 },
            message_handler,
        }
    }
    pub async fn run(&mut self) -> ! {
        loop {
            if let Ok((id, message)) = self.com.try_read::<Message>().await {
                defmt_or_log::info!("Got message: {:?}", id);
                let resp = match message {
                    Message::WhoAreYou => Response::IAm(self.device_identifier.clone()),
                    Message::GetMotorState => self.message_handler.get_motor_state().await,
                    Message::ResetMotor => self.message_handler.reset_motor().await,
                    Message::MoveMotor { x } => self.message_handler.move_motor(x).await,
                    Message::GetPeripheralsState => self.message_handler.get_peripherals_state().await,
                    Message::Water { cooldown_ms } => self.message_handler.water(cooldown_ms).await,
                    Message::Lights { cooldown_ms } => self.message_handler.lights(cooldown_ms).await,
                    Message::Pump { cooldown_ms } => self.message_handler.pump(cooldown_ms).await,
                    Message::Plow { cooldown_ms } => self.message_handler.plow(cooldown_ms).await,
                    Message::SetLed { led } => self.message_handler.set_led(led).await,
                };
                if let Err(e) = self.com.send(resp, id).await {
                    defmt_or_log::info!("Sending response gave error: {:?}", e);
                }
            }
        }
    }
}
