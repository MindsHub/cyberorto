extern crate std;
use std::sync::mpsc::{self, Receiver, Sender};

use rand::{rngs::ThreadRng, thread_rng, Rng};

use crate::Serial;

pub struct Testable {
    tx: Sender<u8>,
    rx: Receiver<u8>,
    error_rate: f64,
    omission_rate: f64,
    rng: ThreadRng,
}
impl Testable {
    pub fn new(error_rate: f64, omission_rate: f64) -> (Self, Self) {
        let (master_tx, slave_rx) = mpsc::channel::<u8>();
        let (slave_tx, master_rx) = mpsc::channel::<u8>();
        let master = Self {
            tx: master_tx,
            rx: master_rx,
            error_rate,
            omission_rate,
            rng: thread_rng(),
        };
        let slave = Self {
            tx: slave_tx,
            rx: slave_rx,
            error_rate,
            omission_rate,
            rng: thread_rng(),
        };
        (master, slave)
    }
}
impl Serial for Testable {
    fn read(&mut self) -> Option<u8> {
        self.rx.try_recv().ok()
    }

    fn write(&mut self, buf: u8) -> bool {
        let buf = if self.rng.gen_bool(self.error_rate) {
            self.rng.gen()
        } else {
            buf
        };
        if self.rng.gen_bool(1.0 - self.omission_rate) {
            let _ = self.tx.send(buf).is_ok();
        }

        true
    }
}
