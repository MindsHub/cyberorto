#![allow(dead_code)]
use core::{cell::{Cell, RefCell}, future::Future, task::Poll};
use arduino_common::Timer;
use arduino_hal::pac::TC0;
use avr_device::interrupt::Mutex;
use panic_halt as _;


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

const MILLIS_INCREMENT: u64 = 1;//PRESCALER * TIMER_COUNTS / 16000;

static MILLIS_COUNTER: Mutex<Cell<u64>> =
    Mutex::new(Cell::new(0));
static TIMER0: Mutex<RefCell<Option<TC0>>> = Mutex::new(RefCell::new(None));

#[avr_device::interrupt(atmega328p)]
fn TIMER0_COMPA() {
    avr_device::interrupt::free(|cs| {
        let counter_cell = MILLIS_COUNTER.borrow(cs);
        let counter = counter_cell.get();
        counter_cell.set(counter + MILLIS_INCREMENT);
    })
}

pub fn millis() -> u64 {
    avr_device::interrupt::free(|cs| MILLIS_COUNTER.borrow(cs).get())
}
pub fn micros()->u64{
    avr_device::interrupt::free(|cs| {
        let v = TIMER0.borrow(cs).borrow_mut().as_mut().unwrap().tcnt0.read().bits() as u64;
        v+ MILLIS_COUNTER.borrow(cs).get()*1000
    })
}

pub struct MillisTimer0;
impl MillisTimer0{
    pub fn new(tc0: arduino_hal::pac::TC0)->Self{
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
            *TIMER0.borrow(cs).borrow_mut()=Some(tc0);
        });
        MillisTimer0
    }
}
impl Timer for  MillisTimer0{
    fn ms_from_start(&self)->u64 {
        millis()
    }
}



pub struct Wait{
    end: u64,
}

impl Wait{
    pub fn from_millis(m: u64)->Self{
        Self{
            end: micros()+m*1000,
        }
    }
    pub fn from_micros(m: u64)->Self{
        Self{
            end: micros()+m,
        }
    }
}
impl Future for Wait{
    type Output=u64;

    fn poll(self: core::pin::Pin<&mut Self>, _cx: &mut core::task::Context<'_>) -> Poll<Self::Output> {
        let diff = micros();
        if let Some(x) = diff.checked_sub(self.end){
            Poll::Ready(x)
        }else{
            Poll::Pending
        }
    }
}