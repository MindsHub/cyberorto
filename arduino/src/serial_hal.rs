use core::cell::RefCell;
use core::future::Future;
use core::task::Poll;

use arduino_common::AsyncSerial;
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
    out_buffer: VecDeque<u8, BUF_SIZE>,
    input_buffer: VecDeque<u8, BUF_SIZE>,
    usart: USART0,
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

        let inp = SerialInner::new(usart);
        interrupt::free(|cs| {
            SERIAL_INNER.borrow(cs).replace(Some(inp));
        });

        SerialHAL
    }
}

impl SerialHAL {
    fn read(&mut self) -> Option<u8> {
        interrupt::free(|cs| {
            if let Some(serial) = SERIAL_INNER.borrow(cs).borrow_mut().as_mut() {
                serial.input_buffer.pop_front()
            } else {
                None
            }
        })
    }
    fn write(&mut self, buf: u8) -> bool {
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
}

// ASYNC SERIAL

struct AsyncSerialRead<'a> {
    s: &'a mut SerialHAL,
}
impl<'a> Future for AsyncSerialRead<'a> {
    type Output = Option<u8>;

    fn poll(
        mut self: core::pin::Pin<&mut Self>,
        _cx: &mut core::task::Context<'_>,
    ) -> Poll<Self::Output> {
        if let Some(v) = self.s.read() {
            Poll::Ready(Some(v))
        } else {
            Poll::Pending
        }
    }
}

struct AsyncSerialWrite<'a> {
    s: &'a mut SerialHAL,
    to_send: u8,
}

impl<'a> Future for AsyncSerialWrite<'a> {
    type Output = bool;

    fn poll(
        mut self: core::pin::Pin<&mut Self>,
        _cx: &mut core::task::Context<'_>,
    ) -> Poll<Self::Output> {
        let v = self.to_send;
        if self.s.write(v) {
            Poll::Ready(true)
        } else {
            Poll::Pending
        }
    }
}

impl AsyncSerial for SerialHAL {
    fn read(&mut self) -> impl Future<Output = Option<u8>> {
        AsyncSerialRead { s: self }
    }

    fn write(&mut self, buf: u8) -> impl Future<Output = bool> {
        AsyncSerialWrite {
            s: self,
            to_send: buf,
        }
    }
}
