
#[tokio::main]
async fn main(){
    let port = serialport::new("/dev/ttyACM0", 115200)
            .timeout(Duration::from_millis(3))
            .parity(serialport::Parity::None)
            .stop_bits(serialport::StopBits::One)
            .flow_control(serialport::FlowControl::None)
            .open_native()
            .expect("Failed to open port"); 
}