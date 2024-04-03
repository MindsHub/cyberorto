#![no_std]
#![no_main]
#![feature(abi_avr_interrupt)]
#![feature(noop_waker)]


use core::{future::Future, pin::pin, task::{Context, Waker}};

use arduino_common::{Comunication, Timer};

use millis::{MillisTimer0, Wait};
use panic_halt as _;
use serial_hal::SerialHAL;

mod serial_hal;
mod millis;


async fn wait_sec(){
    Wait::from_millis(1000).await;
}

fn pooller(){
    //let p = pin!(t);
    let w = Waker::noop();
    let mut cx = Context::from_waker(&w);
    let t = wait_sec();
    let mut w = pin!(t);
    while let core::task::Poll::Pending = w.as_mut().poll(&mut cx){
        
    }
}



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
    let serial = SerialHAL::new(serial, led);
    let mut com = Comunication::new(serial);
    //enable interrupts
    unsafe { avr_device::interrupt::enable() };

    //let mut buf = [0u8; 30];
    let x = MillisTimer0::new(dp.TC0);
    /*let x = x.ms_from_start();
    let x = x.to_le_bytes();
    for x in x{
        serial.write(x);
    }*/
    
    loop {
        
        com.send_serialize(arduino_common::Response::Wait { ms: x.ms_from_start() });
        pooller();
        //arduino_hal::delay_ms(1000);
        //until there are bytes, read them
        //let mut index = 0;

        /*while let Some(readen) = { serial.read() } {
            buf[index] = readen;
            index += 1;
            if index >= 30 {
                break;
            }
        }

        for c in &buf[0..index] {
            serial.write(*c);
        }*/
    }
}
