use std::time::Duration;

use arduino_common::prelude::*;
use serialport::{SerialPort, TTYPort};
use tokio::{sync::Mutex, time::{sleep, Instant}};
#[tokio::main]
async fn main(){
    let port = serialport::new("/dev/ttyACM0", 115200)
        .timeout(Duration::from_millis(3))
        .parity(serialport::Parity::None)
        .stop_bits(serialport::StopBits::One)
        .flow_control(serialport::FlowControl::None)
        .open_native()
        .expect("Failed to open port");
    let master: Master<
        TTYPort,
        StdSleeper,
        Mutex<InnerMaster<TTYPort, StdSleeper>>,
    > = Master::new(port, 4000, 1);
    let mut state = true;
    sleep(Duration::from_millis(3000)).await;
    let mut count =0;
    let i = Instant::now();
    loop{
        state = !state;
        
        let x = master.set_led(state).await;
        count+=1;
        //sleep(Duration::from_millis(1000)).await;
        println!("recv {:?} {}", x, count as f32/i.elapsed().as_secs_f32());

    }
}