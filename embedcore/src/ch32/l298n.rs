use core::f32::consts::PI;

use ch32_hal::{
    gpio::{AnyPin, Level, Output, Speed},
    timer::{Channel, GeneralInstance16bit, simple_pwm::SimplePwm},
};

use micromath::F32Ext;

pub struct MotorPhase<'a, Timer: GeneralInstance16bit> {
    en1: Output<'a>,
    en2: Output<'a>,
    pwm: SimplePwm<'a, Timer>,
    max: f32,
    ch: Channel,
}
impl<'a, Timer: GeneralInstance16bit> MotorPhase<'a, Timer> {
    pub fn new(en1: AnyPin, en2: AnyPin, mut pwm: SimplePwm<'a, Timer>, ch: Channel) -> Self {
        let en1 = Output::new(en1, Level::High, Speed::Low);
        let en2 = Output::new(en2, Level::Low, Speed::Low);
        let max = pwm.get_max_duty() as f32;
        pwm.enable(ch);
        Self {
            en1,
            en2,
            pwm,
            max,
            ch,
        }
    }
    pub fn set_value(&mut self, mut value: f32) {
        //let start=value;
        if value > 0.0 {
            self.en1.set_high();
            self.en2.set_low();
        } else {
            self.en1.set_low();
            self.en2.set_high();
            value = -value;
        }

        value = ((value) * self.max).max(0.0).min(self.max);
        self.pwm.set_duty(self.ch, value as u32)
        //self.pwm.set_duty(self.ch, self.max as u32 /2);
    }
    pub fn discrete_set(&mut self, value: i8) {
        self.pwm.set_duty(self.ch, self.max as u32);
        match value {
            1 => {
                self.en1.set_high();
                self.en2.set_low();
            }
            0 => {
                self.en1.set_low();
                self.en2.set_low();
            }
            _ => {
                self.en1.set_low();
                self.en2.set_high();
            }
        }
    }
}

pub struct Stepper<'a, Timer1: GeneralInstance16bit, Timer2: GeneralInstance16bit> {
    m1: MotorPhase<'a, Timer1>,
    m2: MotorPhase<'a, Timer2>,
}
impl<'a, Timer1: GeneralInstance16bit, Timer2: GeneralInstance16bit> Stepper<'a, Timer1, Timer2> {
    pub fn new(motor1: MotorPhase<'a, Timer1>, motor2: MotorPhase<'a, Timer2>) -> Self where {
        Self {
            m1: motor1,
            m2: motor2,
        }
    }
    //set phase from 0 to 1
    pub fn set_phase(&mut self, x: f32) {
        // normalize
        let x = x.clamp(0., 1.) * 2.0 * PI;
        let phase1 = x.sin();
        let phase2 = x.cos();
        self.m1.set_value(phase1);
        self.m2.set_value(phase2);
    }
    pub fn set_discrete_phase(&mut self, x: u8) {
        match x {
            0 => {
                self.m1.discrete_set(1);
                self.m2.discrete_set(0)
            }
            1 => {
                self.m1.discrete_set(0);
                self.m2.discrete_set(1)
            }
            2 => {
                self.m1.discrete_set(-1);
                self.m2.discrete_set(0)
            }
            _ => {
                self.m1.discrete_set(0);
                self.m2.discrete_set(-1)
            }
        }
    }
}
