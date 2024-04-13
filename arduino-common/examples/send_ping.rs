use std::time::Duration;

use arduino_common::prelude::*;
use serialport::SerialPort;
use tokio::sync::Mutex;
#[tokio::main]
async fn main() {
    let port = serialport::new("/dev/ttyACM0", 115200)
        .timeout(Duration::from_millis(2))
        .parity(serialport::Parity::None)
        .stop_bits(serialport::StopBits::One)
        .flow_control(serialport::FlowControl::None)
        .open()
        .expect("Failed to open port");
    let mut master: Master<
        Box<dyn SerialPort>,
        StdSleeper,
        Mutex<InnerMaster<Box<dyn SerialPort>, StdSleeper>>,
    > = Master::new(port, 1000, 20);
    let mut ok = 0;
    for _ in 0..10000 {
        if master.who_are_you().await.is_ok() {
            ok += 1;
        } else {
            println!("Nope");
        }
    }
    println!("{ok}");
}
