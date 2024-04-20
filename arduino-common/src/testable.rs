extern crate std;

use std::boxed::Box;
use std::sync::Arc;
use std::vec::Vec;
//use std::sync::mpsc::{self, Receiver, Sender};
use rand::{rngs::SmallRng, Rng, SeedableRng};
use tokio::sync::{
    mpsc::{self, Receiver, Sender},
    Mutex,
};

use crate::prelude::*;
pub type TestMaster<Serial> =
    Master<Serial, tokio::time::Sleep, Mutex<InnerMaster<Serial, tokio::time::Sleep>>>;

pub struct Testable {
    tx: Sender<u8>,
    rx: Receiver<u8>,
    error_rate: f64,
    omission_rate: f64,
    rng: SmallRng,
}

impl Testable {
    pub fn new(error_rate: f64, omission_rate: f64) -> (Self, Self) {
        let (master_tx, slave_rx) = mpsc::channel::<u8>(1000);
        let (slave_tx, master_rx) = mpsc::channel::<u8>(1000);
        let master = Self {
            tx: master_tx,
            rx: master_rx,
            error_rate,
            omission_rate,
            rng: SmallRng::from_entropy(), //StdRng::from_rng(OsRng).unwrap(),
        };
        let slave = Self {
            tx: slave_tx,
            rx: slave_rx,
            error_rate,
            omission_rate,
            rng: SmallRng::from_entropy(), //StdRng::from_rng(OsRng).unwrap(),
        };
        (master, slave)
    }
}

impl AsyncSerial for Testable {
    async fn read(&mut self) -> u8 {
        self.rx.recv().await.unwrap()
    }

    async fn write(&mut self, buf: u8) {
        let buf = if self.rng.gen_bool(self.error_rate) {
            self.rng.gen()
        } else {
            buf
        };
        if self.rng.gen_bool(1.0 - self.omission_rate) {
            let _ = self.tx.send(buf).await;
        }
    }
}

pub struct Dummy {
    led_state: &'static Mutex<bool>,
}
impl MessagesHandler for Dummy {
    async fn set_led(&mut self, state: bool) -> Option<Response> {
        *self.led_state.lock().await = state;
        Some(Response::Done)
    }
    async fn move_motor(&mut self, _x: f32) -> Option<Response> {
        Some(Response::Wait { ms: 100 })
    }
    async fn poll(&mut self) -> Option<Response> {
        Some(Response::Done)
    }
}
impl Default for Dummy {
    fn default() -> Self {
        Self {
            led_state: Box::leak(Box::new(Mutex::new(false))),
        }
    }
}
#[derive(Default)]
pub struct MessageRecorderSlave {
    pub incoming: Vec<Message>,
    //outgoing: Vec<Response>,
}

impl MessagesHandler for Arc<std::sync::Mutex<MessageRecorderSlave>> {
    async fn move_motor(&mut self, x: f32) -> Option<Response> {
        self.lock().unwrap().incoming.push(Message::MoveMotor { x });
        Some(Response::Wait { ms: 10 })
    }
    async fn reset_motor(&mut self) -> Option<Response> {
        self.lock().unwrap().incoming.push(Message::ResetMotor);
        Some(Response::Wait { ms: 10 })
    }
    async fn poll(&mut self) -> Option<Response> {
        self.lock().unwrap().incoming.push(Message::Poll);
        Some(Response::Done)
    }
    async fn water(&mut self, ms: u64) -> Option<Response> {
        self.lock()
            .unwrap()
            .incoming
            .push(Message::Water { duration_ms: ms });
        Some(Response::Done)
    }
    async fn lights(&mut self, ms: u64) -> Option<Response> {
        self.lock()
            .unwrap()
            .incoming
            .push(Message::Lights { duration_ms: ms });
        Some(Response::Done)
    }
    async fn pump(&mut self, ms: u64) -> Option<Response> {
        self.lock()
            .unwrap()
            .incoming
            .push(Message::Pump { duration_ms: ms });
        Some(Response::Done)
    }
    async fn plow(&mut self, ms: u64) -> Option<Response> {
        self.lock()
            .unwrap()
            .incoming
            .push(Message::Plow { wait_ms: ms });
        Some(Response::Done)
    }
    async fn set_led(&mut self, state: bool) -> Option<Response> {
        self.lock()
            .unwrap()
            .incoming
            .push(Message::SetLed { led: state });
        Some(Response::Done)
    }
}
pub fn new_testable_slave<Serial: AsyncSerial, Sleeper: Sleep>(
    serial: Serial,
    name: [u8; 10],
) -> Slave<Serial, Sleeper, Arc<std::sync::Mutex<MessageRecorderSlave>>> {
    Slave::new(
        serial,
        3,
        name,
        Arc::new(std::sync::Mutex::new(MessageRecorderSlave::default())),
    )
}

#[cfg(test)]
mod test {
    extern crate std;
    use crate::prelude::*;
    use core::time::Duration;
    use std::sync::Arc;

    use super::Dummy;

    async fn init_test(
        timeout_us: u64,
    ) -> (
        TestMaster<Testable>,
        Slave<Testable, tokio::time::Sleep, Dummy>,
    ) {
        let (master, slave) = Testable::new(0.0, 0.0);
        let master: TestMaster<Testable> = Master::new(master, timeout_us, 10);
        //let state = Box::new(Mutex::new(BotState::default()));
        //let state = &*Box::leak(state);
        let slave: Slave<Testable, tokio::time::Sleep, _> =
            Slave::new(slave, 10, b"ciao      ".clone(), Dummy::default());

        (master, slave)
    }

    #[tokio::test]
    async fn test_led_set() {
        let (master, mut slave) = init_test(10).await;
        let led_state = slave.message_handler.led_state;
        assert!(!*led_state.lock().await);
        let _ = tokio::spawn(async move { slave.run().await });
        master.set_led(true).await.unwrap();
        assert!(*led_state.lock().await);
        master.set_led(false).await.unwrap();
        assert!(!*led_state.lock().await);
    }
    #[tokio::test]
    async fn test_who_are_you() {
        let (master, mut slave) = init_test(10).await;
        let _ = tokio::spawn(async move { slave.run().await });
        let (name, version) = master.who_are_you().await.unwrap();
        assert_eq!(name, b"ciao      ".clone());
        assert_eq!(version, 0);
    }
    #[tokio::test]
    async fn test_move_to() {
        let (master, mut slave) = init_test(10).await;
        let _ = tokio::spawn(async move { slave.run().await });
        master.move_to(0.0).await.unwrap();
    }

    #[tokio::test]
    async fn test_blocking() {
        let (master, mut slave) = init_test(10000).await;
        let _ = tokio::spawn(async move { slave.run().await });
        let master = Arc::new(master);
        let m1 = master.clone();
        let q = tokio::spawn(async move { m1.move_to(1.0).await });
        tokio::time::sleep(Duration::from_millis(10)).await;
        let (name, version) = master.who_are_you().await.unwrap();
        assert_eq!(name, b"ciao      ".clone());
        assert_eq!(version, 0);
        assert!(!q.is_finished());
        //state.mut_lock().await.command = None;
        let res = q.await.unwrap();
        assert_eq!(res, Ok(()))
    }

    #[tokio::test]
    async fn test_timeout() {
        let (master, slave) = Testable::new(0.0, 1.0);
        let master: TestMaster<Testable> = Master::new(master, 10, 10);
        let mut slave: Slave<Testable, tokio::time::Sleep, _> =
            Slave::new(slave, 10, b"ciao      ".clone(), Dummy::default());
        tokio::time::sleep(Duration::from_millis(10)).await;
        let _ = tokio::spawn(async move { slave.run().await });
        let ret = master.who_are_you().await;
        assert_eq!(ret, Err(()));
    }
}
