#![no_std]
#![no_main]
#![feature(abi_avr_interrupt)]
#![feature(noop_waker)]
#![feature(impl_trait_in_assoc_type)]
#![allow(non_snake_case)]
use atmega_hal::prelude::_unwrap_infallible_UnwrapInfallible;
use avr_tc1_embassy_time::{define_interrupt, init_system_time};
use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};

define_interrupt!(atmega328p);

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
//define_interrupt!(atmega328p);
/*struct MyDriver{

}
impl Driver for MyDriver{
    fn now(&self) -> u64 {
        todo!()
    }

    unsafe fn allocate_alarm(&self) -> Option<embassy_time_driver::AlarmHandle> {
        todo!()
    }

    fn set_alarm_callback(&self, alarm: embassy_time_driver::AlarmHandle, callback: fn(*mut ()), ctx: *mut ()) {
        todo!()
    }

    fn set_alarm(&self, alarm: embassy_time_driver::AlarmHandle, timestamp: u64) -> bool {
        todo!()
    }
}
time_driver_impl!(static DRIVER: MyDriver = MyDriver{});*/

fn encoder(){

}

#[embassy_executor::main(entry = "arduino_hal::entry")]
async fn main(_s: Spawner) -> ! {
    let mut dp = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);
    let p3 = pins.d3.into_pull_up_input();
    let p4 = pins.d4.into_pull_up_input();
    let p5 = pins.d5.into_pull_up_input();
    let p6 = pins.d6.into_pull_up_input();

    init_system_time(&mut dp.TC1);
    let mut serial = arduino_hal::default_serial!(dp, pins, 57600);
    loop{
        Timer::after(Duration::from_millis(100)).await;
        ufmt::uwriteln!(&mut serial, "test").unwrap_infallible();
    }
}


