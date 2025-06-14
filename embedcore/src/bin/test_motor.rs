use std::time::Duration;

use defmt_or_log::info;
use embedcore::protocol::cyber::Master;
use tokio::time::{Instant, sleep};

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
    let master: Master<SerialStream> = Master::new(port, 40000, 1);
    let mut state = true;
    sleep(Duration::from_millis(3000)).await;
    println!("Starting");
    let mut count = 0;
    let i = Instant::now();
    loop {
        state = !state;

        let x = master.set_led(state).await;
        count += 1;
        println!("set");
        sleep(Duration::from_millis(1000)).await;
        //info!("recv {:?} {}", x, count as f32 / i.elapsed().as_secs_f32());
    }
}
