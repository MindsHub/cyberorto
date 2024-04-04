use core::time::Duration;

use tokio::time::sleep;

use crate::Sleep;


impl Sleep for tokio::time::Sleep{
    fn await_us(us: u64)->Self {
        sleep(Duration::from_micros(us))
    }
}