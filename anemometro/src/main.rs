#![no_std]
#![no_main]
#![feature(abi_avr_interrupt)]
#![feature(noop_waker)]

use core::{
    future::Future,
    pin::pin,
    task::{Context, Waker},
};

use arduino_common::{no_std::SingleCoreMutex, traits:: MutexTrait, BotState, SlaveBot};


use arduino_hal::port::{mode::Output, Pin, PinOps};

use millis::{init_millis, millis, Wait};
use panic_halt as _;
use serial_hal::SerialHAL;

/// module containing all timings stuff (init, millis, micros...)
pub mod millis;
/// module containing all serial stugg (init, async traits, buffer dimensions...)
pub mod serial_hal;


///Main entry point
#[arduino_hal::entry]
fn main() -> ! {
    //getting peripherals
    let dp = arduino_hal::Peripherals::take().unwrap();
    let pins = ardUino_hal::pins!(dp);

    // set led pin to low
    let mut led = pins.d13.into_output();
    led.set_low();
    // extract usart, and init it
    let serial = dp.USART0;
    let serial = SerialHAL::new(serial);
    //let com = Comunication::new(serial);
    //enable interrupts
    unsafe { avr_device::interrupt::enable() };
    
    init_millis(dp.TC0);
    let pin_ane = pins.d1.into_floating_input();
    
    fn calc_time() {
        let mut time: u64 = 0;
        while pin_ane.islow() {
            time = pin_ane.is_high() - pin_ane.islow();
        }
    }

    let time: u64 = calc_time();
    serial

    pin_ane.is_high();
}
