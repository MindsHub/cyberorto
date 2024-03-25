use core::cell::RefCell;

use arduino_hal::prelude::_embedded_hal_serial_Read;
use arduino_hal::prelude::_embedded_hal_serial_Write;
use arduino_hal::{
    hal::port::{PD0, PD1},
    pac::USART0,
    port::{
        mode::{Input, Output},
        Pin,
    },
    Usart,
};
use avr_device::interrupt::{self, Mutex};
use circular_buffer::CircularBuffer;

type CurUsart = Usart<USART0, Pin<Input, PD0>, Pin<Output, PD1>>;
type UsartReader = arduino_hal::usart::UsartReader<USART0, Pin<Input, PD0>, Pin<Output, PD1>>;
type UsartWriter = arduino_hal::usart::UsartWriter<USART0, Pin<Input, PD0>, Pin<Output, PD1>>;

const BUF_SIZE: usize = 20;

static SERIAL_READER: Mutex<RefCell<Option<SerialReader>>> = Mutex::new(RefCell::new(None));
static SERIAL_WRITER: Mutex<RefCell<Option<SerialWriter>>> = Mutex::new(RefCell::new(None));
//static mut SERIAL_READER: Mutex<Option<SerialWr>> = Mutex::new(None);
struct SerialWriter {
    buffer: CircularBuffer<BUF_SIZE, u8>,
    usart: UsartWriter,
    overflowed: bool,
    sending: bool,
}
impl SerialWriter {
    fn new(usart: UsartWriter) -> Self {
        SerialWriter {
            usart,
            buffer: CircularBuffer::new(),
            overflowed: false,
            sending: false,
        }
    }
}
#[avr_device::interrupt(atmega328p)]
fn USART_TX() {
    interrupt::free(|cs| {
        if let Some(serial) = SERIAL_WRITER.borrow(cs).borrow_mut().as_mut() {
            if let Some(s) = serial.buffer.pop_front() {
                let _ = serial.usart.write(s);
            } else {
                serial.sending = false;
            };
        }
    });
}

struct SerialReader {
    usart: UsartReader,
    buffer: CircularBuffer<BUF_SIZE, u8>,
    overflowed: bool,
}
impl SerialReader {
    pub fn new(usart: UsartReader) -> Self {
        SerialReader {
            usart,
            buffer: CircularBuffer::new(),
            overflowed: false,
        }
    }
}
#[avr_device::interrupt(atmega328p)]
fn USART_RX() {
    interrupt::free(|cs| {
        if let Some(reader) = SERIAL_READER.borrow(cs).borrow_mut().as_mut() {
            let b = reader.usart.read().unwrap();
            if reader.buffer.try_push_back(b).is_err() {
                reader.overflowed = true;
                //x.alarm_led.set_high();
            }



            //TODO remove me
            /*if let Some(x) = SERIAL_WRITER.borrow(cs).borrow_mut().as_mut() {
                x.buffer.push_back(b);
                if x.sending {
                    return;
                }

                let Some(t) = x.buffer.pop_front() else {
                    return;
                };
                let _ = x.usart.write(t);
                x.sending = true;
            }*/
        }
    });
}

pub struct SerialHAL;
impl SerialHAL {
    pub fn new(usart: CurUsart) -> Self {
        let (reader, writer) = usart.split();
        let reader = SerialReader::new(reader);
        let writer = SerialWriter::new(writer);
        interrupt::free(|cs| {
            SERIAL_READER.borrow(cs).replace(Some(reader));
            SERIAL_WRITER.borrow(cs).replace(Some(writer));
        });

        SerialHAL
    }
    pub fn status(&mut self) ->(bool, bool){
        interrupt::free(
            |cs| {
                let first = if let Some(serial) = SERIAL_READER.borrow(cs).borrow_mut().as_mut() {
                    serial.overflowed
                }else {true};
                let second = if let Some(serial) = SERIAL_WRITER.borrow(cs).borrow_mut().as_mut() {
                    serial.overflowed
                }else {true};
                (first, second)
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
            if let Some(serial) = SERIAL_READER.borrow(cs).borrow_mut().as_mut(){
                serial.buffer.pop_front()
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
            if let Some(x) = SERIAL_WRITER.borrow(cs).borrow_mut().as_mut() {
                if x.buffer.try_push_back(buf).is_err(){
                    x.overflowed=true;
                }
                if x.sending {
                    return;
                }

                let Some(t) = x.buffer.pop_front() else {
                    return;
                };
                let _ = x.usart.write(t);
                x.sending = true;
            }

            // let serial = SERIAL_WRITER.borrow(cs).get_mut().as_mut().unwrap();
            //serial.add_write(buf);
        });
    }
}
