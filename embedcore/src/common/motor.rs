use defmt_or_log::trace;
use embassy_time::Timer;

use crate::prelude::*;

pub struct Motor<E: EncoderTrait, D: DiscreteDriver> {
    pub encoder: E,
    pub driver: D,
    pub rotation: bool,
    shift: i32,
}

impl<E: EncoderTrait, D: DiscreteDriver> Motor<E, D> {
    pub fn new(encoder: E, driver: D, rotation: bool) -> Self {
        Self {
            encoder,
            driver,
            rotation,
            shift: 0,
        }
    }
    pub async fn align(&mut self, current: f32, wait_time: f32) {
        for i in 0..(self.get_microstep() * 4) {
            self.set_phase(i as u8, current);
            Timer::after_millis(1).await;
        }
        self.set_phase(0, current);
        Timer::after_millis((1000.0 * wait_time) as u64).await;
        let value = self.read();
        self.set_phase(0, 0.0);

        let shift = value.rem_euclid(D::MICROSTEP as i32 * 4);
        trace!("value = {}, shift = {}", value, shift);
        if shift > D::MICROSTEP as i32 * 2 {
            self.shift(D::MICROSTEP as i32 * 4 - shift);
        } else {
            self.shift(-shift);
        }
    }
    fn shift(&mut self, amount: i32) {
        if self.rotation {
            self.shift += amount;
        } else {
            self.shift -= amount;
        }
    }
    pub async fn set_current(&mut self, current: f32) {
        let mut pos = self.read();
        if current > 0.0 {
            pos += D::MICROSTEP as i32;
        } else {
            pos -= D::MICROSTEP as i32;
        }
        self.set_phase(pos.rem_euclid(D::MICROSTEP as i32 * 4) as u8, current.abs());
    }
}

impl<E: EncoderTrait, D: DiscreteDriver> EncoderTrait for Motor<E, D> {
    fn read(&mut self) -> i32 {
        let mut v = self.encoder.read() + self.shift;
        if !self.rotation {
            v = -v;
        }
        v
    }
}

impl<E: EncoderTrait, D: DiscreteDriver> DiscreteDriver for Motor<E, D> {
    const MICROSTEP: usize = D::MICROSTEP;

    fn set_phase(&mut self, phase: u8, current: f32) {
        self.driver.set_phase(phase, current);
    }
}

pub mod test {
    use embassy_time::{Duration, Instant, Ticker, Timer};

    use super::{DiscreteDriver, EncoderTrait, Motor};

    pub async fn test_basic_movement<D: DiscreteDriver, E: EncoderTrait>(
        motor: &mut Motor<E, D>,
        current: f32,
    ) {
        Timer::after_millis(100).await;
        motor.align(current, 1.0).await;

        let start = motor.read();
        move_rotation(motor, 0.5, true, current).await;
        motor.set_phase(0, current);
        Timer::after_millis(100).await;
        let duration = motor.read() - start;
        if !(3980..=4020).contains(&duration) {
            motor.set_phase(0, 0.0);
            defmt_or_log::panic!("expected from 3980 and 4020, got {}", duration);
        }

        let start = motor.read();
        move_rotation(motor, 0.5, false, current).await;
        motor.set_phase(0, current);
        Timer::after_millis(100).await;
        let duration = motor.read() - start;
        if !(-4020..=-3980).contains(&duration) {
            motor.set_phase(0, 0.0);
            defmt_or_log::panic!("expected from -4020 and -3980, got {}", duration);
        }
        motor.set_phase(0, 0.0);
    }
    pub async fn move_rotation<D: DiscreteDriver, E: EncoderTrait>(
        motor: &mut Motor<E, D>,
        speed: f32,
        direction: bool,
        current: f32,
    ) {
        let mut t = Ticker::every(Duration::from_hz((4000.0 * speed) as u64));
        for i in 0..4000 {
            let phase: u8 = (i % (motor.get_microstep() * 4)) as u8;
            if direction {
                motor.set_phase(phase, current);
            } else {
                motor.set_phase(motor.get_microstep() as u8 * 4 - phase, current);
            }
            t.next().await;
        }
    }
    pub async fn test_max_speed<D: DiscreteDriver, E: EncoderTrait>(
        motor: &mut Motor<E, D>,
        direction: bool,
    ) -> f32 {
        motor.align(2.0, 1.0).await;
        let start = motor.read();
        let increment = if direction { 20 } else { -20 };
        let start_time = Instant::now();
        let mut e = Instant::now();
        let mut percorsi = 0;
        //info!("start = {} {} {}", start, increment, start_time.elapsed().as_micros());
        while (-200000..=200000).contains(&percorsi) {
            let cur = motor.read();
            if e.elapsed().as_millis() > 1000 {
                //info!("cur = {}", cur);
                e = Instant::now();
            }
            motor.set_phase(
                ((cur + increment).rem_euclid(D::MICROSTEP as i32 * 4)) as u8,
                2.0,
            );
            embassy_futures::yield_now().await;
            percorsi = cur - start;
        }

        let ret = percorsi as f32 / 4000.0 * 1_000_000.0 / start_time.elapsed().as_micros() as f32;
        motor.set_phase(0, 2.0);

        Timer::after_millis(100).await;
        motor.set_phase(0, 0.0);
        ret
    }
}
