#![no_std]
#![no_main]
#![feature(abi_avr_interrupt)]

use panic_halt as _;
use serial_v2::{Serial, SerialHAL};

//static mut USART: Mutex<Option<Usart<USART0, Pin<Input, PD0>, Pin<Output, PD1>, MHz16>>> = Mutex::new(None);

//use zerocopy::{AsBytes, FromBytes};
/*
#[repr(C, packed)]
#[derive(FromBytes, AsBytes, Debug, Clone, Copy, PartialEq)]
enum TestEnum{
    Lol(u32),
    H(u32),
}*/
//mod serial_hal;
mod serial_v2;
#[arduino_hal::entry]
fn main() -> ! {
    //getting peripherals
    let dp = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);
    //let t = dp.USART0.;
    //setting uart
    let serial =dp.USART0;
    
    //let mut serial = arduino_hal::default_serial!(dp, pins, 115200);
    
    /*arduino_hal::Usart::new(p, rx, tx, baudrate)
    serial.listen(Event::RxComplete);
    serial.listen(Event::TxComplete);*/
    //serial.listen(Event::DataRegisterEmpty);
    let mut led = pins.d13.into_output();
    led.set_low();
    let mut serial = SerialHAL::new(serial, led);

    //enable interrupts
    unsafe { avr_device::interrupt::enable() };
    //let mut buf = [0u8; 30];
    loop {
        arduino_hal::delay_ms(1);
        
        //let mut len =0;
        while let Some(readen) = serial.read(){
           // buf[len]=readen;
            serial.write(readen);
           // len+=1;
            /*if len ==30{
                break;
            }*/
        }

        
    }
}
