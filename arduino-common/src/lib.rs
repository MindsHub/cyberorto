#![no_std]


use panic_halt as _;
use serialmessage::SerMsg;
#[arduino_hal::entry]
fn main() -> ! {
    let dp = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);


    let mut led = pins.d13.into_output();

    loop {
        led.toggle();
        
        //let send_data_vec= [1, 2, 3, 4];
        //let send_msg = SerMsg::create_msg_arr(&send_data_vec, 1).unwrap();
        
        //arduino_hal::delay_ms(send_msg.1 as u16);
        arduino_hal::delay_ms(1000);
    }
}
