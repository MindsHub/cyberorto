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


use millis::{init_millis, Wait};
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
    /*let pins = arduino_hal::pins!(dp);

    // set led pin to low
    let mut led = pins.d13.into_output();
    led.set_low();*/

    // extract usart, and init it
    let serial = dp.USART0;
    let serial = SerialHAL::new(serial);
    //let com = Comunication::new(serial);

    //enable interrupts
    unsafe { avr_device::interrupt::enable() };


    init_millis(dp.TC0);

    //create context for async
    let w = Waker::noop();
    let mut cx = Context::from_waker(&w);
    
    let state = SingleCoreMutex::new(BotState::new());
    
    let mut s: SlaveBot<SerialHAL, Wait, _> = SlaveBot::new(serial, 100, b"ciao      ".clone(), &state);
    let mut serial_async = pin!(async move {
        
        s.run().await;
    });

    //main loop
    loop {
        let _ = serial_async.as_mut().poll(&mut cx);
    }
    /*loop{
        let _ =pin!(serial.write(b'r')).poll(&mut cx);
        delay_ms(10);
    }*/
}
