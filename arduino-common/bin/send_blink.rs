
#[tokio::main]
async fn main(){
    let port = tokio_serial::new("/dev/ttyACM0", 115200)
            .timeout(Duration::from_millis(3))
            .parity(tokio_serial::Parity::None)
            .stop_bits(tokio_serial::StopBits::One)
            .flow_control(tokio_serial::FlowControl::None)
            .open_native_async()
            .expect("Failed to open port"); 
}