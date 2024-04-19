use std::{
    io::Write,
    thread::sleep,
    time::{Duration, Instant},
};

use arduino_common::prelude::*;
use tokio_serial::{SerialPortBuilderExt, SerialStream};

#[tokio::main]
async fn main() {
    let mut port = tokio_serial::new("/dev/ttyACM0", 115200)
        .timeout(Duration::from_millis(100))
        .parity(tokio_serial::Parity::None)
        .stop_bits(tokio_serial::StopBits::One)
        .flow_control(tokio_serial::FlowControl::None)
        .open_native_async()
        .expect("Failed to open port");
    let _ = port.flush();
    sleep(Duration::from_secs_f32(1.58));
    let mut comunication: Comunication<SerialStream, tokio::time::Sleep> = Comunication::new(port, 100);
    let first_time = Instant::now();
    let mut first: Option<Response> = None;
    while first.is_none() {
        first = comunication.try_read().await.unwrap().1;
    }
    let first = first.unwrap();
    let first = match first {
        arduino_common::Response::Wait { ms } => ms,
        _ => 0,
    };

    loop {
        if let Some((_, m)) = comunication.try_read().await {
            let time_elapsed = first_time.elapsed().as_millis();
            let ex_elapsed = match m {
                arduino_common::Response::Wait { ms } => ms - first,
                _ => 0,
            };
            println!(
                "{:?}  {}-{} {:.4}",
                m,
                time_elapsed,
                ex_elapsed,
                time_elapsed as f64 / ex_elapsed as f64
            );
        }
    }
}
