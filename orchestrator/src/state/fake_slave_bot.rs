use std::sync::{Arc, Mutex};

use arduino_common::{comunication::Comunication, std::StdSleeper, Message, Response};
use serialport::TTYPort;


pub struct FakeSlaveBot {
    com: Comunication<TTYPort, StdSleeper>,
    name: [u8; 10],
    data: Arc<Mutex<FakeSlaveBotData>>,
}

#[derive(Debug, Clone)]
pub struct FakeSlaveBotData {
    pub received_messages: Vec<Message>,
}

impl FakeSlaveBot {
    pub fn new(serial: TTYPort, name: [u8; 10]) -> FakeSlaveBot {
        FakeSlaveBot {
            com: Comunication::new(serial, 3),
            name,
            data: Arc::new(Mutex::new(FakeSlaveBotData{
                received_messages: Vec::new(),
            })),
        }
    }

    pub fn get_data_ref(&self) -> Arc<Mutex<FakeSlaveBotData>> {
        self.data.clone()
    }

    pub async fn run(&mut self) -> ! {
        loop {
            if let Some((id, message)) = self.com.try_read::<Message>().await {
                self.data.lock().unwrap().received_messages.push(message.clone());

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
                    _ => {
                        self.com.send(Response::Done, id).await;
                    }
                }
            }
        }
    }
}
