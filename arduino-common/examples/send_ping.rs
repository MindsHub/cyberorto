use std::time::Duration;

use arduino_common::prelude::*;
use tokio::sync::Mutex;
use tokio_serial::{SerialPortBuilderExt, SerialStream};

#[tokio::main]
async fn main() {
    let port = tokio_serial::new("/dev/ttyACM0", 115200)
        .timeout(Duration::from_millis(2))
        .parity(tokio_serial::Parity::None)
        .stop_bits(tokio_serial::StopBits::One)
        .flow_control(tokio_serial::FlowControl::None)
        .open_native_async()
        .expect("Failed to open port");
    let master: Master<
        SerialStream,
        tokio::time::Sleep,
        Mutex<InnerMaster<SerialStream, tokio::time::Sleep>>,
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
