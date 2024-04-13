extern crate std;

//use std::sync::mpsc::{self, Receiver, Sender};
use rand::{rngs::SmallRng, Rng, SeedableRng};
use tokio::sync::{
    mpsc::{self, Receiver, Sender},
    Mutex,
};

use crate::prelude::*;
pub type TestMaster<Serial> = Master<Serial, StdSleeper, Mutex<InnerMaster<Serial, StdSleeper>>>;

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

#[cfg(test)]
mod test {
    extern crate std;
    use crate::{prelude::*, BotState};
    use std::boxed::Box;
    use tokio::sync::Mutex;

    async fn init_test() -> (
        TestMaster<Testable>,
        SlaveBot<'static, Testable, StdSleeper, Mutex<BotState>>,
    ) {
        let (master, slave) = Testable::new(0.0, 0.0);
        let master: TestMaster<Testable> = Master::new(master, 10, 10);
        let state = Box::new(Mutex::new(BotState::new()));
        let state = &*Box::leak(state);
        let slave: SlaveBot<Testable, StdSleeper, _> =
            SlaveBot::new(slave, 10, b"ciao      ".clone(), state);

        (master, slave)
    }

    #[tokio::test]
    async fn test() {
        let (_master, mut slave) = init_test().await;
        let _ = tokio::spawn(async move { slave.run().await });
    }
    #[tokio::test]
    async fn test_led_set() {
        let (master, mut slave) = init_test().await;
        let refer = slave.state;
        assert!(!refer.mut_lock().await.led);
        let _ = tokio::spawn(async move { slave.run().await });
        master.set_led(true).await.unwrap();
        assert!(refer.mut_lock().await.led);
        
    }

}
