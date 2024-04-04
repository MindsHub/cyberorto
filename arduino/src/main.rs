#![no_std]
#![no_main]
#![feature(abi_avr_interrupt)]
#![feature(noop_waker)]

use core::{
    future::Future,
    pin::pin,
    task::{Context, Waker},
};

use arduino_common::Slave;

use millis::{MillisTimer0, Wait};
use panic_halt as _;
use serial_hal::SerialHAL;

mod millis;
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
    let serial = SerialHAL::new(serial);
    //let com = Comunication::new(serial);

    //enable interrupts
    unsafe { avr_device::interrupt::enable() };


    let _x = MillisTimer0::new(dp.TC0);

    //create context for async
    let w = Waker::noop();
    let mut cx = Context::from_waker(&w);

    //create future serial for pooling
    let mut serial_async = pin!(async move {
        let mut s: Slave<SerialHAL, Wait> = Slave::new(serial, 100, b"ciao      ".clone());
        s.run().await;
    });

    //main loop
    loop {
        let _ = serial_async.as_mut().poll(&mut cx);
    }
}
