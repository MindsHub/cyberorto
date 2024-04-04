use std::{
    thread::sleep,
    time::{Duration, Instant},
};

use arduino_common::{Comunication, Response};
use serialport::{ClearBuffer, SerialPort};
use tokio::time::Sleep;

fn flush(port: &mut Box<dyn SerialPort>) {
    port.flush().unwrap();
    let to_read = port.bytes_to_read().unwrap();
    if to_read == 0 {
        return;
    }
    let mut buf: Vec<u8> = vec![0u8; to_read as usize];
    port.read_exact(buf.as_mut_slice()).unwrap();
    port.clear(ClearBuffer::Input).unwrap();
    port.clear(ClearBuffer::Output).unwrap();
}

#[tokio::main]
async fn main() {
    let mut port = serialport::new("/dev/ttyACM0", 115200)
        .timeout(Duration::from_millis(100))
        .parity(serialport::Parity::None)
        .stop_bits(serialport::StopBits::One)
        .flow_control(serialport::FlowControl::None)
        .open()
        .expect("Failed to open port");

    flush(&mut port);
    sleep(Duration::from_secs_f32(1.58));
    let mut comunication: Comunication<Box<dyn SerialPort>, Sleep> = Comunication::new(port, 100);
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
