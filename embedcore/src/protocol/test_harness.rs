extern crate std;

use std::boxed::Box;
use std::sync::Arc;
use std::vec::Vec;
//use std::sync::mpsc::{self, Receiver, Sender};
use rand::{Rng, SeedableRng, rngs::SmallRng};
use tokio::sync::{
    Mutex,
    mpsc::{self, Receiver, Sender},
};

use crate::protocol::comunication::CommunicationError;

use super::{AsyncSerial, cyber::*};
pub type TestMaster<Serial> = Master<Serial>;

pub struct Testable {
    tx: Sender<u8>,
    rx: Receiver<u8>,
    error_rate: f64,
    omission_rate: f64,
    random: SmallRng,
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
            random: SmallRng::from_os_rng(), //StdRng::from_rng(OsRng).unwrap(),
        };
        let slave = Self {
            tx: slave_tx,
            rx: slave_rx,
            error_rate,
            omission_rate,
            random: SmallRng::from_os_rng(), //StdRng::from_rng(OsRng).unwrap(),
        };
        (master, slave)
    }
}

impl AsyncSerial for Testable {
    async fn read(&mut self) -> Result<u8, CommunicationError> {
        Ok(self.rx.recv().await.unwrap())
    }

    async fn write(&mut self, buf: u8) -> Result<(), CommunicationError> {
        let buf = if self.random.random_bool(self.error_rate) {
            self.random.random()
        } else {
            buf
        };
        if self.random.random_bool(1.0 - self.omission_rate) {
            self.tx.send(buf).await.unwrap();
        }
        Ok(())
    }
}

pub struct Dummy {
    pub led_state: &'static Mutex<bool>,
}
impl MessagesHandler for Dummy {
    async fn set_led(&mut self, state: bool) -> Response {
        *self.led_state.lock().await = state;
        Response::Ok
    }
    async fn move_motor(&mut self, _x: f32) -> Response {
        Response::Ok
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
    pub led_state: bool,
    //outgoing: Vec<Response>,
}

impl MessagesHandler for Arc<std::sync::Mutex<MessageRecorderSlave>> {
    async fn move_motor(&mut self, x: f32) -> Response {
        self.lock().unwrap().incoming.push(Message::MoveMotor { x });
        Response::Ok
    }
    async fn reset_motor(&mut self) -> Response {
        self.lock().unwrap().incoming.push(Message::ResetMotor);
        Response::Ok
    }

    async fn get_peripherals_state(&mut self) -> Response {
        let mut lock = self.lock().unwrap();
        lock.incoming.push(Message::GetPeripheralsState);
        Response::PeripheralsState(PeripheralsState {
            water: false,
            lights: false,
            pump: false,
            plow: false,
            led: lock.led_state,
        })
    }
    async fn water(&mut self, cooldown_ms: u64) -> Response {
        self.lock()
            .unwrap()
            .incoming
            .push(Message::Water { cooldown_ms });
        Response::Ok
    }
    async fn lights(&mut self, cooldown_ms: u64) -> Response {
        self.lock()
            .unwrap()
            .incoming
            .push(Message::Lights { cooldown_ms });
        Response::Ok
    }
    async fn pump(&mut self, cooldown_ms: u64) -> Response {
        self.lock()
            .unwrap()
            .incoming
            .push(Message::Pump { cooldown_ms });
        Response::Ok
    }
    async fn plow(&mut self, cooldown_ms: u64) -> Response {
        self.lock()
            .unwrap()
            .incoming
            .push(Message::Plow { cooldown_ms });
        Response::Ok
    }
    async fn set_led(&mut self, state: bool) -> Response {
        let mut lock = self.lock().unwrap();
        lock.led_state = state;
        lock.incoming.push(Message::SetLed { led: state });
        Response::Ok
    }
}
pub fn new_testable_slave<Serial: AsyncSerial>(
    serial: Serial,
    name: [u8; 10],
) -> Slave<Serial, Arc<std::sync::Mutex<MessageRecorderSlave>>> {
    Slave::new(
        serial,
        name,
        Arc::new(std::sync::Mutex::new(MessageRecorderSlave::default())),
    )
}
