extern crate std;
use std::println;

use crate::{AsyncSerial, Comunication, Message, Response, Sleep};

pub struct Master<Serial: AsyncSerial, Sleeper: Sleep> {
    id: u8,
    com: Comunication<Serial, Sleeper>,
    //TODO add id
}
impl<Serial: AsyncSerial, Sleeper: Sleep> Master<Serial, Sleeper> {
    pub fn new(serial: Serial, timeout_us: u64) -> Self {
        Self {
            com: Comunication::new(serial, timeout_us),
            id: 0,
        }
    }
    async fn send(&mut self, m: Message) -> bool {
        //print!("Sending {:?}", m);
        self.id = self.id.wrapping_add(1);
        self.com.send(m, self.id).await
    }
    async fn try_read(&mut self) -> Option<Response> {
        let (id, resp) = self.com.try_read::<Response>().await?;
        if id == self.id {
            Some(resp)
        } else {
            None
        }
    }
    pub async fn move_to(&mut self, x: f32, y: f32, z: f32) -> Result<(), ()> {
        let m = Message::Move { x, y, z };
        //retry only 10 times
        'complete: for i in 0..1 {
            // send Move
            if !self.send(m.clone()).await {
                continue;
            }
            let id = self.id;

            while let Some((id_read, msg)) = self.com.try_read::<Response>().await {
                if id_read != id {
                    continue;
                }
                match msg {
                    Response::Wait { ms } => {
                        Sleeper::await_us(ms * 1000).await;
                        if !self.send(Message::Pool { id }).await {
                            continue 'complete;
                        }
                    }
                    Response::Done => {
                        println!("resend {i}");
                        return Ok(());
                    }
                    _ => {}
                }
            }
        }
        Err(())
    }
    pub async fn who_are_you(&mut self) -> Result<([u8; 10], u8), ()> {
        for _ in 0..50 {
            if !self.send(Message::WhoAreYou).await {
                continue;
            }
            let id = self.id;

            while let Some((id_read, msg)) = self.com.try_read::<Response>().await {
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

#[cfg(test)]
mod test {
    extern crate std;
    use std::println;

    use serialmessage::{ParseState, SerMsg};
    use tokio::time::Sleep;

    use crate::{testable::Testable, AsyncSerial, Message, Slave};

    use super::Master;

    #[tokio::test]
    async fn test_master() {
        let (master, slave) = Testable::new(0.1, 0.00);
        let mut slave: Slave<Testable, Sleep> = Slave::new(slave, 0, b"ciao      ".clone());
        let q = tokio::spawn(async move { slave.run().await });
        let mut master: Master<Testable, Sleep> = Master::new(master, 0);
        let mut ok = 0;
        let total = 1000;
        for _ in 0..total {
            if Ok((b"ciao      ".clone(), 0)) == master.who_are_you().await {
                println!("OK");
                ok += 1
            } else {
                println!("NO");
            }
        }
        q.abort();
        panic!("{ok}/{total}");
    }
    #[tokio::test]
    async fn test_accuracy() {
        let (master, mut slave) = Testable::new(0.1, 0.00);
        let mut master: Master<Testable, Sleep> = Master::new(master, 0);
        let mut ok = 0;
        for _ in 0..10000 {
            master
                .send(crate::Message::Move {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                })
                .await;
            let mut buf = [0u8; 19];
            for c in &mut buf {
                *c = slave.read().await;
            }
            let mut msg = SerMsg::new();
            if let (ParseState::DataReady, _) = msg.parse_read_bytes(&buf) {
                let data = msg.return_read_data();
                if let Ok(Message::Move { x, y, z }) = postcard::from_bytes::<Message>(data) {
                    if x == 0.0 && y == 0.0 && z == 0.0 {
                        //correct

                        ok += 1;
                    } else {
                        println!("no");
                    }
                }
            }
        }
        panic!("{}", ok);
    }
}
