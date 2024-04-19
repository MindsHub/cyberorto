use std::time::Duration;

use arduino_common::prelude::*;
use tokio::{
    sync::Mutex,
    time::{sleep, Instant},
};
use tokio_serial::{SerialPortBuilderExt, SerialStream};
#[tokio::main]
async fn main() {
    let port = tokio_serial::new("/dev/ttyACM0", 115200)
        .timeout(Duration::from_millis(3))
        .parity(tokio_serial::Parity::None)
        .stop_bits(tokio_serial::StopBits::One)
        .flow_control(tokio_serial::FlowControl::None)
        .open_native_async()
        .expect("Failed to open port");
    let master: Master<
        SerialStream,
        tokio::time::Sleep,
        Mutex<InnerMaster<SerialStream, tokio::time::Sleep>>,
    > = Master::new(port, 4000, 1);
    let mut state = true;
    sleep(Duration::from_millis(3000)).await;
    let mut count = 0;
    let i = Instant::now();
    loop {
        state = !state;

        let x = master.set_led(state).await;
        count += 1;
        //sleep(Duration::from_millis(1000)).await;
        println!("recv {:?} {}", x, count as f32 / i.elapsed().as_secs_f32());
    }
}
