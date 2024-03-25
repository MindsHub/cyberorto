use core::cell::RefCell;

use arduino_hal::clock::MHz16;
use arduino_hal::hal::port::PB5;

use arduino_hal::hal::usart::BaudrateExt;
use arduino_hal::usart::Baudrate;
use arduino_hal::{
    pac::USART0,
    port::{
        mode::Output,
        Pin,
    },
};
use avr_device::interrupt::{self, Mutex};
use circular_buffer::CircularBuffer;

const BUF_SIZE: usize = 20;

static SERIAL_INNER: Mutex<RefCell<Option<SerialInner>>> = Mutex::new(RefCell::new(None));
//static mut SERIAL_READER: Mutex<Option<SerialWr>> = Mutex::new(None);
struct SerialInner {
    out_buffer: CircularBuffer<BUF_SIZE, u8>,
    input_buffer: CircularBuffer<BUF_SIZE, u8>,
    usart: USART0,
    overflowed: bool,
    sending: bool,
    led: Pin<Output, PB5>
}
impl SerialInner {
    fn new(usart: USART0, led: Pin<Output, PB5>) -> Self {
        SerialInner {
            usart,
            out_buffer: CircularBuffer::new(),
            input_buffer: CircularBuffer::new(),
            led,
            overflowed: false,
            sending: false,
        }
    }
}
#[avr_device::interrupt(atmega328p)]
fn USART_TX() {
    interrupt::free(|cs| {
        if let Some(serial) = SERIAL_INNER.borrow(cs).borrow_mut().as_mut() {
            if let Some(s) = serial.out_buffer.pop_front() {
                serial.usart.udr0.write(|w| w.bits(s) );
            } else {
                //serial.usart.ucsr0b.write(|w|{w.udrie0().clear_bit()});
                serial.sending = false;
            };
        }
    });
}

#[avr_device::interrupt(atmega328p)]
fn USART_RX() {
    interrupt::free(|cs| {
        if let Some(reader) = SERIAL_INNER.borrow(cs).borrow_mut().as_mut() {
            let b = reader.usart.udr0.read().bits();
            if reader.input_buffer.try_push_back(b).is_err() {
                reader.overflowed = true;
                //x.alarm_led.set_high();
            }
            reader.led.toggle();
        }
    });
}

pub struct SerialHAL;
impl SerialHAL {
    pub fn new(usart: USART0, led: Pin<Output, PB5>) -> Self {
        //set baudrate
        let c: Baudrate<MHz16> = BaudrateExt::into_baudrate(115200);
        usart.ubrr0.write(|w| w.bits(c.ubrr)); //((16000000/8/115200)-1)/2)
        usart.ucsr0a.write(|w| w.u2x0().bit(c.u2x));

        usart.ucsr0b.write(|w| w
            .rxcie0().set_bit()
            //.udrie0().set_bit()//.set_bit()
            .txcie0().set_bit()
            .txen0().set_bit()
            .rxen0().set_bit());
        
        usart.ucsr0c.write(|w| w
            //set 1 stop bit
            .usbs0().stop1()
            //set 0 parity bit
            .upm0().disabled()
            //8 bit
            .ucsz0().chr8()
            .umsel0().usart_async());
        

        let inp = SerialInner::new(usart, led);
        interrupt::free(|cs| {
            SERIAL_INNER.borrow(cs).replace(Some(inp));
        });

        SerialHAL
    }
    pub fn status(&mut self) ->bool{
        interrupt::free(
            |cs| {
                if let Some(serial) = SERIAL_INNER.borrow(cs).borrow_mut().as_mut() {
                    serial.overflowed
                }else {true}
            },
        )
    }
}

pub trait Serial {
    fn read(&mut self) -> Option<u8>;
    //fn advance_buffer(&mut self, to_remove: usize);
    fn write(&mut self, buf: u8);
}
impl Serial for SerialHAL {
    fn read(&mut self) -> Option<u8> {
        interrupt::free(|cs| {
            if let Some(serial) = SERIAL_INNER.borrow(cs).borrow_mut().as_mut(){
                //serial.led.toggle();
                serial.input_buffer.pop_front()
            }else{
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
    fn write(&mut self, buf: u8) {
        interrupt::free(|cs| {
            if let Some(x) = SERIAL_INNER.borrow(cs).borrow_mut().as_mut() {
                if x.out_buffer.try_push_back(buf).is_err(){
                    x.overflowed=true;
                }
                if x.sending {
                    return;
                }

                let Some(t) = x.out_buffer.pop_front() else {
                    return;
                };
                //x.usart.ucsr0b.write(|w|{w.udrie0().set_bit()});
                x.usart.udr0.write(|w| w.bits(t));
                //enable 
               
                x.sending = true;
            }

            // let serial = SERIAL_WRITER.borrow(cs).get_mut().as_mut().unwrap();
            //serial.add_write(buf);
        });
    }
}
