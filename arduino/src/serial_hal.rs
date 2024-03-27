
use core::cell::RefCell;

use arduino_common::Serial;
use arduino_hal::clock::MHz16;
use arduino_hal::hal::port::PB5;

use arduino_hal::hal::usart::BaudrateExt;
use arduino_hal::usart::Baudrate;
use arduino_hal::{
    pac::USART0,
    port::{mode::Output, Pin},
};
use avr_device::interrupt::{self, Mutex};
use fixed_queue::VecDeque;



const BUF_SIZE: usize = 20;

static SERIAL_INNER: Mutex<RefCell<Option<SerialInner>>> = Mutex::new(RefCell::new(None));
//static mut SERIAL_READER: Mutex<Option<SerialWr>> = Mutex::new(None);
struct SerialInner {
    out_buffer: VecDeque<u8, BUF_SIZE>,
    input_buffer: VecDeque<u8, BUF_SIZE>,
    usart: USART0,
    overflowed: bool,
    sending: bool,
    led: Pin<Output, PB5>,
}
impl SerialInner {
    fn new(usart: USART0, led: Pin<Output, PB5>) -> Self {
        SerialInner {
            usart,
            out_buffer: VecDeque::new(),
            input_buffer: VecDeque::new(),
            led,
            overflowed: false,
            sending: false,
        }
    }
}
#[avr_device::interrupt(atmega328p)]
fn USART_UDRE() {
    interrupt::free(|cs| {
        if let Some(serial) = SERIAL_INNER.borrow(cs).borrow_mut().as_mut() {
            serial.led.set_high();
            //serial.usart.ucsr0b.modify(|w| w.)
            //serial.usart.udr0.write(|w| w.bits(b'r') );
            //serial.usart.ucsr0b.modify(|_, w|{w.udrie0().clear_bit()});
            if let Some(s) = serial.out_buffer.pop_front() {
                serial.usart.udr0.write(|w| w.bits(s));
            } else {
                serial.usart.ucsr0b.modify(|_, w| w.udrie0().clear_bit());
                serial.sending = false;
            };
            serial.led.set_low();
        }
    });
}

#[avr_device::interrupt(atmega328p)]
fn USART_RX() {
    interrupt::free(|cs| {
        if let Some(reader) = SERIAL_INNER.borrow(cs).borrow_mut().as_mut() {
            reader.led.set_high();
            let b = reader.usart.udr0.read().bits();
            if reader.input_buffer.push_back(b).is_err() {
                reader.overflowed = true;
                //reader.led.set_high();
                //x.alarm_led.set_high();
            }
            //reader.out_buffer.push_back(b);
            //reader.usart.ucsr0b.modify(|_, w|{w.udrie0().set_bit()});

            //reader.led.toggle();
            reader.led.set_low();
        }
    });
}

pub struct SerialHAL;
impl SerialHAL {
    pub fn new(usart: USART0, led: Pin<Output, PB5>) -> Self {
        //set baudrate
        let c: Baudrate<MHz16> = BaudrateExt::into_baudrate(115200);
        usart.ubrr0.write(|w| w.bits(c.ubrr));
        usart.ucsr0a.write(|w| w.u2x0().bit(c.u2x));

        usart.ucsr0b.write(|w| {
            w.rxcie0()
                .set_bit()
                //.udrie0().clear_bit()
                //.txcie0().set_bit()
                .txen0()
                .set_bit()
                .rxen0()
                .set_bit()
        });

        usart.ucsr0c.write(|w| {
            w
                //set 1 stop bit
                .usbs0()
                .stop1()
                //set 0 parity bit
                .upm0()
                .disabled()
                //8 bit
                .ucsz0()
                .chr8()
                .umsel0()
                .usart_async()
        });

        let inp = SerialInner::new(usart, led);
        interrupt::free(|cs| {
            SERIAL_INNER.borrow(cs).replace(Some(inp));
        });

        SerialHAL
    }
}

impl Serial for SerialHAL {
    fn read(&mut self) -> Option<u8> {
        interrupt::free(|cs| {
            if let Some(serial) = SERIAL_INNER.borrow(cs).borrow_mut().as_mut() {
                serial.input_buffer.pop_front()
            } else {
                None
            }
        })
    }

    /* fn advance_buffer(&mut self, to_remove: usize) {
        interrupt::free(|cs| {
            if let Some(serial) = SERIAL_READER.borrow(cs).borrow_mut().as_mut() {
                let len = serial.buffer.len();
                serial.buffer.truncate_front(len - to_remove);
            }
        });
    }*/
    fn write(&mut self, buf: u8) -> bool {
        interrupt::free(|cs| {
            if let Some(serial) = SERIAL_INNER.borrow(cs).borrow_mut().as_mut() {
                if serial.out_buffer.push_back(buf).is_err() {
                    serial.overflowed = true;
                    serial.led.set_high();
                    return false;
                }
                serial.usart.ucsr0b.modify(|_, w| w.udrie0().set_bit());
            }
            true
        })
    }
}
