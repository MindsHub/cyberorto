use std::time::Duration;

use tokio::{self, time::Sleep};
use serialport::SerialPort;
use arduino_common::master::Master;
#[tokio::main]
async fn main(){
    let port = serialport::new("/dev/ttyACM0", 115200)
        .timeout(Duration::from_millis(2))
        .parity(serialport::Parity::None)
        .stop_bits(serialport::StopBits::One)
        .flow_control(serialport::FlowControl::None)
        .open()
        .expect("Failed to open port");
    let mut master: Master<Box<dyn SerialPort>, Sleep> = Master::new(port, 1000);
    let mut ok =0;
    for _ in 0..10000{
        if let Ok(_) = master.who_are_you().await{
            ok+=1;
        }else{
            println!("Nope");
        }
    }
    println!("{ok}");
    
}