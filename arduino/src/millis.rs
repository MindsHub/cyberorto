#![allow(dead_code)]
use arduino_common::Timer;
use arduino_hal::pac::TC0;
use avr_device::interrupt::Mutex;
use core::{
    cell::{Cell, RefCell},
    future::Future,
    task::Poll,
};
use panic_halt as _;
//use this const to configure the precision and intervals of interrupts

// Possible Values:
//
// ╔═══════════╦══════════════╦═══════════════════╗
// ║ PRESCALER ║ TIMER_COUNTS ║ Overflow Interval ║
// ╠═══════════╬══════════════╬═══════════════════╣
// ║        64 ║          250 ║              1 ms ║
// ║       256 ║          125 ║              2 ms ║
// ║       256 ║          250 ║              4 ms ║
// ║      1024 ║          125 ║              8 ms ║
// ║      1024 ║          250 ║             16 ms ║
// ╚═══════════╩══════════════╩═══════════════════╝
const PRESCALER: u64 = 64;
const TIMER_COUNTS: u64 = 249;

const MILLIS_INCREMENT: u64 = 1; //PRESCALER * TIMER_COUNTS / 16000;

/// shared timer counter. count's the millis, and so it should be good for 2^64/1000 secs or more or less half a milion years
static MILLIS_COUNTER: Mutex<Cell<u64>> = Mutex::new(Cell::new(0));

/// sharing the timer interface, it requires a crytical section for unsafe extract
static TIMER0: Mutex<RefCell<Option<TC0>>> = Mutex::new(RefCell::new(None));

/// timer interrupt, it gets triggered every milliseconds
#[avr_device::interrupt(atmega328p)]
fn TIMER0_COMPA() {
    avr_device::interrupt::free(|cs| {
        let counter_cell = MILLIS_COUNTER.borrow(cs);
        let counter = counter_cell.get();
        counter_cell.set(counter + MILLIS_INCREMENT);
    })
}

/// returns the number of ms from startup (or better, from init)
pub fn millis() -> u64 {
    avr_device::interrupt::free(|cs| MILLIS_COUNTER.borrow(cs).get())
}
/// returns the number of microseconds from startup (or better, from init)
///
/// the maximum resolution is 4 ms, but it could be wronger cause the execution times
pub fn micros() -> u64 {
    avr_device::interrupt::free(|cs| {
        let v = TIMER0
            .borrow(cs)
            .borrow_mut()
            .as_mut()
            .unwrap()
            .tcnt0
            .read()
            .bits() as u64;
        v * 4 + MILLIS_COUNTER.borrow(cs).get() * 1000
    })
}

/// struct used to init and keep track of the timer
pub struct MillisTimer0;
impl MillisTimer0 {
    pub fn new(tc0: arduino_hal::pac::TC0) -> Self {
        // Configure the timer for the above interval (in CTC mode)
        // and enable its interrupt.

        tc0.tccr0a.write(|w| w.wgm0().ctc());
        tc0.ocr0a.write(|w| w.bits(TIMER_COUNTS as u8));
        tc0.tccr0b.write(|w| match PRESCALER {
            8 => w.cs0().prescale_8(),
            64 => w.cs0().prescale_64(),
            256 => w.cs0().prescale_256(),
            1024 => w.cs0().prescale_1024(),
            _ => panic!(),
        });
        tc0.timsk0.write(|w| w.ocie0a().set_bit());

        // Reset the global millisecond counter
        avr_device::interrupt::free(|cs| {
            MILLIS_COUNTER.borrow(cs).set(0);
            *TIMER0.borrow(cs).borrow_mut() = Some(tc0);
        });
        MillisTimer0
    }
}
impl Timer for MillisTimer0 {
    fn ms_from_start(&self) -> u64 {
        millis()
    }
}

/// async future, it returns pending until some ms/micros are elapsed
///
/// it's precision depends on how many times it get's polled
pub struct Wait {
    end: u64,
}

impl Wait {
    /// build wait from how many millis do you want to wait
    pub fn from_millis(m: u64) -> Self {
        Self {
            end: micros() + m * 1000,
        }
    }
    /// build wait from how many micros do you want to wait. is limited in precision from millis
    pub fn from_micros(m: u64) -> Self {
        Self { end: micros() + m }
    }
}
impl Future for Wait {
    type Output = u64;

    fn poll(
        self: core::pin::Pin<&mut Self>,
        _cx: &mut core::task::Context<'_>,
    ) -> Poll<Self::Output> {
        let diff = micros();
        if let Some(x) = diff.checked_sub(self.end) {
            Poll::Ready(x)
        } else {
            Poll::Pending
        }
    }
}
