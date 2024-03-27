#![no_std]
#![no_main]
#![feature(abi_avr_interrupt)]

use arduino_common::Serial;
use panic_halt as _;
use serial_hal::SerialHAL;

mod serial_hal;
#[arduino_hal::entry]
fn main() -> ! {
    //getting peripherals
    let dp = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);

    // set led pin to low
    let mut led = pins.d13.into_output();
    led.set_low();

    // extract usart, and init it
    let serial = dp.USART0;
    let mut serial = SerialHAL::new(serial, led);

    //enable interrupts
    unsafe { avr_device::interrupt::enable() };

    let mut buf = [0u8; 30];
    loop {
        arduino_hal::delay_ms(1);

        //until there are bytes, read them
        let mut index = 0;

        while let Some(readen) = { serial.read() } {
            buf[index] = readen;
            index += 1;
            if index >= 30 {
                break;
            }
        }

        for c in &buf[0..index] {
            serial.write(*c);
        }
    }
}
