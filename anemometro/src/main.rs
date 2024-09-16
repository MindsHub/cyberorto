#![no_std]
#![no_main]
#![feature(abi_avr_interrupt)]
#![feature(noop_waker)]

use millis::init_millis;
use panic_halt as _;

/// module containing all timings stuff (init, millis, micros...)
pub mod millis;


///Main entry point
#[arduino_hal::entry]
fn main() -> ! {
    //getting peripherals
    let dp = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);
    let pin_ane = pins.d2.into_floating_input();
    let mut serial = arduino_hal::default_serial!(dp, pins, 115200);
    //enable interrupts
    unsafe { avr_device::interrupt::enable() };
    init_millis(dp.TC0);

    ufmt::uwriteln!(&mut serial, "{} bytes available", 53);

    /*fn calc_time() {
        let mut time: u64 = 0;
        while pin_ane.islow() {
            time = pin_ane.is_high() - pin_ane.islow();
        }
    }

    let time: u64 = calc_time();*/
    todo!()
}
