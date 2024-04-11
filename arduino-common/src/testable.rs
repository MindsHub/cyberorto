extern crate std;

//use std::sync::mpsc::{self, Receiver, Sender};
use rand::{rngs::SmallRng, Rng, SeedableRng};
use tokio::sync::{mpsc::{self, Receiver, Sender}, Mutex};

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
mod test{
    use crate::prelude::*;

    async fn init_test()->(TestMaster<Testable>, SlaveBot<Testable, StdSleeper>){
        let (master, slave) = Testable::new(0.0, 0.0);
        let master: TestMaster<Testable> = Master::new(master, 10, 10);
        let slave: SlaveBot<Testable, StdSleeper> = SlaveBot::new(slave, 10, b"ciao      ".clone());
        
        (master, slave)
    }
    
    #[tokio::test]
    async fn test(){
        let (master, mut slave) = init_test().await;
        let _  = tokio::spawn(async move{ slave.run().await});
    }
}