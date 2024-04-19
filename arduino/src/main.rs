#![no_std]
#![no_main]
#![feature(abi_avr_interrupt)]
#![feature(noop_waker)]

use core::{
    future::Future,
    pin::pin,
    task::{Context, Waker},
};

use arduino_common::prelude::*;


use arduino_hal::port::{mode::Output, Pin, PinOps};

use millis::{init_millis, Wait};
use panic_halt as _;
use serial_hal::SerialHAL;

/// module containing all timings stuff (init, millis, micros...)
pub mod millis;
/// module containing all serial stugg (init, async traits, buffer dimensions...)
pub mod serial_hal;


struct MHandler<LED: PinOps>{
    status_pin: Pin<Output, LED>,
}
impl<LED: PinOps> MHandler<LED>{
    fn new(p: Pin<Output, LED>)->Self{
        Self { status_pin: p }
    }
}

impl<LED: PinOps> MessagesHandler for MHandler<LED>{
    async fn set_led(&mut self, state: bool)->Option<Response> {
        if state{
            self.status_pin.set_high();
        }else{
            self.status_pin.set_low();
        }
        Some(Response::Done)
    }
}




///Main entry point
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
    
    init_millis(dp.TC0);
    //create context for async
    let w = Waker::noop();
    let mut cx = Context::from_waker(&w);
    
    //let state = SingleCoreMutex::new(BotState::default());
    let message_handler = MHandler::new(led);
    let mut s: Slave<SerialHAL, Wait, _ > = Slave::new(serial, 100, b"ciao      ".clone(), message_handler);

    let mut serial_async = pin!(async move {
        s.run().await;
    });
    //let mut led = led.downgrade();
    //let mut state = pin!(set_state(&state, &mut led));
    
    
    //main loop
    loop {
        let _ = serial_async.as_mut().poll(&mut cx);
        //let _ = state.as_mut().poll(&mut cx);
    }
}
