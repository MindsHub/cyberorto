use core::{cell::RefCell, task::Context};
use core::future::Future;
use core::task::Poll;

use arduino_common::prelude::*;
use arduino_hal::clock::MHz16;

use arduino_hal::hal::usart::BaudrateExt;
use arduino_hal::pac::USART0;
use arduino_hal::usart::Baudrate;
use avr_device::interrupt::{self, Mutex};
use fixed_queue::VecDeque;

/// how bit the input and output buffer should be? it depends on how many times passes from one poll and another
const BUF_SIZE: usize = 30;

/// shared reference to the serial interface + some additional data (buffers, and co.)
static SERIAL_INNER: Mutex<RefCell<Option<SerialInner>>> = Mutex::new(RefCell::new(None));

/// packed way to rapresent data. it should not be visible from outside this module
struct SerialInner {
    /// buffer used for output
    out_buffer: VecDeque<u8, BUF_SIZE>,
    /// buffer used for input
    input_buffer: VecDeque<u8, BUF_SIZE>,
    /// serial interface
    usart: USART0,
    /// has this interface overflowed?
    overflowed: bool,
}
impl SerialInner {
    /// packs the data together
    fn new(usart: USART0) -> Self {
        SerialInner {
            usart,
            out_buffer: VecDeque::new(),
            input_buffer: VecDeque::new(),
            overflowed: false,
        }
    }
}

/// Data register Empty Interrupt
#[avr_device::interrupt(atmega328p)]
fn USART_UDRE() {
    interrupt::free(|cs| {
        if let Some(serial) = SERIAL_INNER.borrow(cs).borrow_mut().as_mut() {
            if let Some(s) = serial.out_buffer.pop_front() {
                serial.usart.udr0.write(|w| w.bits(s));
            } else {
                serial.usart.ucsr0b.modify(|_, w| w.udrie0().clear_bit());
            };
        }
    });
}

/// Received byte interrupt
#[avr_device::interrupt(atmega328p)]
fn USART_RX() {
    interrupt::free(|cs| {
        if let Some(serial) = SERIAL_INNER.borrow(cs).borrow_mut().as_mut() {
            let b = serial.usart.udr0.read().bits();
            if serial.input_buffer.push_back(b).is_err() {
                serial.overflowed = true;
            }
        }
    });
}

/// public, lightweight interface for create and destroy serial comunication
pub struct SerialHAL;
impl SerialHAL {
    /// init a new serial
    pub fn new(usart: USART0) -> Self {
        //set baudrate
        let c: Baudrate<MHz16> = BaudrateExt::into_baudrate(115200);
        usart.ubrr0.write(|w| w.bits(c.ubrr));
        usart.ucsr0a.write(|w| w.u2x0().bit(c.u2x));

        usart.ucsr0b.write(|w| {
            w
                //rx complete interrupt enable
                .rxcie0()
                .set_bit()
                //enable tx
                .txen0()
                .set_bit()
                //enable rx
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
                //set async mode (read while sending)
                .umsel0()
                .usart_async()
        });
        // full the static
        let inp = SerialInner::new(usart);
        interrupt::free(|cs| {
            SERIAL_INNER.borrow(cs).replace(Some(inp));
        });

        SerialHAL
    }
}

impl SerialHAL {
    /// tries to read one byte from serial buffer
    pub fn read(&mut self) -> Option<u8> {
        interrupt::free(|cs| {
            if let Some(serial) = SERIAL_INNER.borrow(cs).borrow_mut().as_mut() {
                serial.input_buffer.pop_front()
            } else {
                None
            }
        })
    }
    /// tries to write on byte into serial buffer. If it is full it return false, on success it returns true
    pub fn write(&mut self, buf: u8) -> bool {
        interrupt::free(|cs| {
            if let Some(serial) = SERIAL_INNER.borrow(cs).borrow_mut().as_mut() {
                if serial.out_buffer.push_back(buf).is_err() {
                    serial.overflowed = true;
                    return false;
                }
                serial.usart.ucsr0b.modify(|_, w| w.udrie0().set_bit());
            }
            true
        })
    }

    /// print a string to the serial, without checking for errors
    pub fn print(&mut self, s: &str)  {
        for c in s.bytes() {
            self.write(c);
        }
    }
    /// print a string to the serial followed by '\n', without checking for errors
    pub fn println(&mut self, s: &str) {
        self.print(s);
        self.write(b'\n');
    }
}


/// Future for await that one byte is readable. 
struct AsyncSerialRead<'a> {
    s: &'a mut SerialHAL,
}
impl<'a> Future for AsyncSerialRead<'a> {
    type Output = u8;

    fn poll(
        mut self: core::pin::Pin<&mut Self>,
        _cx: &mut Context<'_>,
    ) -> Poll<Self::Output> {
        if let Some(v) = self.s.read() {
            Poll::Ready(v)
        } else {
            Poll::Pending
        }
    }
}

/// Future for await that one byte is writable inside the buffer. 
struct AsyncSerialWrite<'a> {
    s: &'a mut SerialHAL,
    to_send: u8,
}

impl<'a> Future for AsyncSerialWrite<'a> {
    type Output = ();

    fn poll(
        mut self: core::pin::Pin<&mut Self>,
        _cx: &mut Context<'_>,
    ) -> Poll<Self::Output> {
        let v = self.to_send;
        if self.s.write(v) {
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    }
}

/// async implementation
impl AsyncSerial for SerialHAL {
    ///returns the read future
    fn read(&mut self) -> impl Future<Output = u8> {
        AsyncSerialRead { s: self }
    }

    ///returns the write future
    fn write(&mut self, buf: u8) -> impl Future<Output = ()> {
        AsyncSerialWrite {
            s: self,
            to_send: buf,
        }
    }
}
