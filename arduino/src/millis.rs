#![allow(dead_code)]
use arduino_common::Sleep;
use arduino_hal::pac::{tc0::tccr0b::CS0_A, TC0};
use avr_device::interrupt::Mutex;
use core::{
    cell::RefCell,
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
const PRESCALER: CS0_A = CS0_A::PRESCALE_64;
const TIMER_COUNTS: u64 = 249;

const MILLIS_INCREMENT: u64 = 1; //PRESCALER * TIMER_COUNTS / 16000;

/// shared timer counter. count's the millis, and so it should be good for 2^64/1000 secs or more or less half a milion years
static MILLIS_COUNTER: Mutex<RefCell<u64>> = Mutex::new(RefCell::new(0));

/// sharing the timer interface, it requires a crytical section for unsafe extract
static TIMER0: Mutex<RefCell<Option<TC0>>> = Mutex::new(RefCell::new(None));

/// timer interrupt, it gets triggered every milliseconds
#[avr_device::interrupt(atmega328p)]
fn TIMER0_COMPA() {
    avr_device::interrupt::free(|cs| {
        let counter_cell = MILLIS_COUNTER.borrow(cs);
        let mut counter = counter_cell.borrow_mut();
        *counter += MILLIS_INCREMENT;
        //counter_cell.set(counter + MILLIS_INCREMENT);
    })
}

/// returns the number of ms from startup (or better, from init)
pub fn millis() -> u64 {
    avr_device::interrupt::free(|cs| *MILLIS_COUNTER.borrow(cs).borrow())
}
/// returns the number of microseconds from startup (or better, from init)
///
/// the maximum resolution is 4 us, but it could be wronger cause the execution times
pub fn micros() -> u64 {
    avr_device::interrupt::free(|cs| {
        let v = TIMER0
            .borrow(cs)
            .borrow_mut()
            .as_mut()
            .unwrap()
            .tcnt0
            .read()
            .bits();
        v as u64 * 4 + *MILLIS_COUNTER.borrow(cs).borrow() * 1000
    })
}


/// init timer
pub fn init_millis(tc0: arduino_hal::pac::TC0) {
    // Configure the timer for the above interval (in CTC mode)
    // and enable its interrupt.

    // set waveform mode
    tc0.tccr0a.write(|w| w.wgm0().ctc());

    // set timer counts
    tc0.ocr0a.write(|w| w.bits(TIMER_COUNTS as u8));

    // set prescaler, only some values are possible
    tc0.tccr0b.write(|w| w.cs0().variant(PRESCALER));

    // activate interrupt
    tc0.timsk0.write(|w| w.ocie0a().set_bit());

    // Reset the global millisecond counter
    avr_device::interrupt::free(|cs| {
        *MILLIS_COUNTER.borrow(cs).borrow_mut() =0;
        *TIMER0.borrow(cs).borrow_mut() = Some(tc0);
    });
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
impl Sleep for Wait{
    fn await_us(us: u64)->Self {
        Self::from_micros(us)
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
