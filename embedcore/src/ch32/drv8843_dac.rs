#![allow(non_snake_case)]

use crate::{DiscreteDriver, common::math::ComputedSin};
use ch32_hal::{
    dac::{DacChannel, Value},
    gpio::{Input, Output},
    peripherals::DAC1,
};
use embassy_time::Timer;

#[derive(Clone)]
pub enum DecayType {
    Fast,
    Mixed,
    Slow,
}
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
pub struct Drv8843Dac<'a, const MICROSTEP: usize> {
    reset: Output<'a>,
    nFault: Input<'a>,
    decay: Output<'a>,
    AIN1: Output<'a>,
    AIN2: Output<'a>,
    BIN1: Output<'a>,
    BIN2: Output<'a>,
    vrefa: DacChannel<'a, DAC1, 1>,
    vrefb: DacChannel<'a, DAC1, 2>,
    decay_type: DecayType,

    computed_sin: ComputedSin<MICROSTEP>,
}
impl<'a, const MICROSTEP: usize> Drv8843Dac<'a, MICROSTEP> {
    pub fn new(
        mut reset: Output<'a>,
        nFault: Input<'a>,
        mut decay: Output<'a>,
        AIN1: Output<'a>,
        AIN2: Output<'a>,
        mut vrefa: DacChannel<'a, DAC1, 1>,
        mut vrefb: DacChannel<'a, DAC1, 2>,
        BIN1: Output<'a>,
        BIN2: Output<'a>,
    ) -> Self {
        decay.set_low();

        let computed_sin = ComputedSin::new();
        reset.set_high();
        vrefa.enable();
        vrefb.enable();
        Self {
            reset,
            nFault,
            decay,
            AIN1,
            AIN2,
            BIN1,
            BIN2,
            vrefa,
            vrefb,
            decay_type: DecayType::Slow,
            computed_sin,
        }
    }
    pub async fn reset(&mut self) {
        self.reset.set_low();
        Timer::after_millis(1).await;
        self.reset.set_high();
    }
    pub fn is_faulty(&self) -> bool {
        self.nFault.is_low()
    }
    /*pub fn destroy(Self { nFault, decay, pinA, pinB, ..}: Self)->(Input<'a>, Output<'a>,  SimplePwm<'a, TimerA>, SimplePwm<'a, TimerB>){
        (nFault, decay, pinA, pinB)
    }*/
    pub fn set_decay(&mut self, decay_type: DecayType) {
        self.decay_type = decay_type;
        match self.decay_type {
            DecayType::Fast => self.decay.set_high(),
            DecayType::Mixed => todo!(),
            DecayType::Slow => self.decay.set_low(),
        }
    }
    pub fn get_decay(&self) -> &DecayType {
        &self.decay_type
    }
}

impl<'a, const MICROSTEP: usize> DiscreteDriver for Drv8843Dac<'a, MICROSTEP> {
    const MICROSTEP: usize = MICROSTEP;
    fn set_phase(&mut self, phase: u8, current: f32) {
        let sin_phase = self.computed_sin.sin(phase as usize);
        let cos_phase = self.computed_sin.cos(phase as usize);
        // TODO remove 1.25, is a correction factor for the prototype
        let sin_duty = current / 2.2 * sin_phase;
        let cos_duty = current / 2.2 * cos_phase;
        self.low_level_current_set(sin_duty, cos_duty);
    }
}

impl<'a, const MICROSTEP: usize> Drv8843Dac<'a, MICROSTEP> {
    /// pass on time (from 0.0 to 1.0)
    pub fn low_level_current_set(&mut self, a: f32, b: f32) {
        // set a

        const FULL: u16 = 0b111111111111;
        let to_set_a = (a.abs() * FULL as f32).floor() as u16;
        if a > 0.0 {
            self.AIN1.set_high();
            self.AIN2.set_low();
        } else {
            self.AIN1.set_low();
            self.AIN2.set_high();
        }
        self.vrefa.set(Value::Bit12Right(to_set_a));

        // set b
        let to_set_b = (b.abs() * FULL as f32).floor() as u16;
        if b > 0.0 {
            self.BIN1.set_high();
            self.BIN2.set_low();
        } else {
            self.BIN1.set_low();
            self.BIN2.set_high();
        }
        self.vrefb.set(Value::Bit12Right(to_set_b));
    }
}
