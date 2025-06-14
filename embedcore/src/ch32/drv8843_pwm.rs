#![allow(non_snake_case)]

use crate::{DiscreteDriver, common::math::ComputedSin};
use ch32_hal::{
    gpio::{AnyPin, Input, Output},
    timer::{Channel, GeneralInstance16bit, simple_pwm::SimplePwm},
};

use micromath::F32Ext;

/**Datasheet: <https://www.ti.com/lit/ds/symlink/drv8843.pdf?ts=1718652873770&ref_url=https%253A%252F%252Fwww.ti.com%252Fsitesearch%252Fen-us%252Fdocs%252Funiversalsearch.tsp%253FlangPref%253Den-US>
This is v0.1, current is set writing with a pwm on vref, and direction writing on IN1 and IN2.
BI1 to GND
BI0 to GND
AI1 to GND
AI0 to GND
BIN2
BIN1
AIN1
AIN2
DECAY -> (during current chopping)3.3v fast, 0v slow, unconnected = mixed
nFAULT -> input (really needed?)
nSLEEP -> to 3.3
nRESET -> to 3.3
*/
pub struct Drv8843Pwm<
    'a,
    TimerA: GeneralInstance16bit,
    TimerB: GeneralInstance16bit,
    const MICROSTEP: usize,
> {
    reset: Output<'a>,
    nFault: Input<'a>,
    _decay: Input<'a>,
    AIN: (Output<'a>, Output<'a>),
    BIN: (Output<'a>, Output<'a>),
    pina: (Channel, SimplePwm<'a, TimerA>),
    pinb: (Channel, SimplePwm<'a, TimerB>),
    //pwms: SimplePwm<'a, Timer>,
    computed_sin: ComputedSin<MICROSTEP>,
}
impl<'a, TimerA: GeneralInstance16bit, TimerB: GeneralInstance16bit, const MICROSTEP: usize>
    Drv8843Pwm<'a, TimerA, TimerB, MICROSTEP>
{
    pub fn new(
        mut reset: Output<'a>,
        nFault: Input<'a>,
        decay: AnyPin,
        AIN: (Output<'a>, Output<'a>),
        BIN: (Output<'a>, Output<'a>),
        mut pwma: SimplePwm<'a, TimerA>,
        mut pwmb: SimplePwm<'a, TimerB>,
        pina: Channel,
        pinb: Channel,
    ) -> Self {
        pwma.enable(pina);
        pwmb.enable(pinb);
        //TODO for good
        //decay
        // decay.set_low();
        reset.set_high();
        let computed_sin = ComputedSin::new();
        Self {
            reset,
            nFault,
            _decay: Input::new(decay, ch32_hal::gpio::Pull::Up),
            AIN,
            BIN,
            pina: (pina, pwma),
            pinb: (pinb, pwmb),
            computed_sin,
        }
    }
    pub fn is_faulty(&self) -> bool {
        self.nFault.is_low()
    }
    pub async fn reset(&mut self) {
        self.reset.set_low();
        embassy_time::Timer::after_millis(1).await;
        self.reset.set_high();
    }
}

impl<'a, TimerA: GeneralInstance16bit, TimerB: GeneralInstance16bit, const MICROSTEP: usize>
    DiscreteDriver for Drv8843Pwm<'a, TimerA, TimerB, MICROSTEP>
{
    const MICROSTEP: usize = MICROSTEP;
    fn set_phase(&mut self, phase: u8, current: f32) {
        let sin_phase = self.computed_sin.sin(phase as usize);
        let cos_phase = self.computed_sin.cos(phase as usize);
        // TODO remove 1.25, is a correction factor for the prototype
        let sin_duty = current / 2.0 * sin_phase;
        let cos_duty = current / 2.0 * cos_phase;
        self.low_level_current_set(sin_duty, cos_duty);
    }
}

impl<'a, TimerA: GeneralInstance16bit, TimerB: GeneralInstance16bit, const MICROSTEP: usize>
    Drv8843Pwm<'a, TimerA, TimerB, MICROSTEP>
{
    /// pass on time (from 0.0 to 1.0)
    pub fn low_level_current_set(&mut self, a: f32, b: f32) {
        // set a
        let to_set_a = (a.abs() * self.pina.1.get_max_duty() as f32).floor() as u32;
        if a > 0.0 {
            self.AIN.0.set_high();
            self.AIN.1.set_low();
        } else {
            self.AIN.0.set_low();
            self.AIN.1.set_high();
        }
        self.pina.1.set_duty(self.pina.0, to_set_a);

        // set b
        let to_set_b = (b.abs() * self.pinb.1.get_max_duty() as f32).floor() as u32;
        if b > 0.0 {
            self.BIN.0.set_high();
            self.BIN.1.set_low();
        } else {
            self.BIN.0.set_low();
            self.BIN.1.set_high();
        }
        //trace!("{} {} {} {} {}", to_set_a, to_set_b, self.AIN.0.is_set_high(), self.BIN.0.is_set_high(), self.pina.1.get_max_duty());
        self.pinb.1.set_duty(self.pinb.0, to_set_b);
    }
}
