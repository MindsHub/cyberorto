use core::f32;

use defmt_or_log::trace;
use embassy_futures::yield_now;
use embassy_time::{Instant, Timer};
use pid::Pid;

use crate::{DiscreteDriver, EncoderTrait};

use crate::common::motor::Motor;

pub struct PidController<E: EncoderTrait, D: DiscreteDriver> {
    pub motor: Motor<E, D>,
    pub pid: Pid<f32>,
    reset_current: f32,
    counter_ups: u64,
}
pub enum CalibrationMode {
    P,
    PI,
    PID,
    Pessen,
    ModerateOvershoot,
    NoOvershoot,
}

impl<E: EncoderTrait, D: DiscreteDriver> PidController<E, D> {
    pub fn new(motor: Motor<E, D>, limit: f32, reset_current: f32) -> Self {
        Self {
            pid: Pid::new(0.0, limit),
            reset_current,
            motor,
            counter_ups: 0,
        }
    }
    /// performs Åström-Hägglund calibration
    pub async fn calibration(
        &mut self,
        calibration_pos: i32,
        mode: CalibrationMode,
    ) -> (f32, f32, f32) {
        trace!("calibration");
        let current = self.reset_current;
        self.motor.align(current, 0.4).await;

        while self.motor.read() < calibration_pos + 2000 {
            self.motor.set_current(current).await;
            yield_now().await;
        }
        trace!("fase 1");
        let mut previous_duration = 0;
        let mut amplitude;

        loop {
            //trace!("fase 2");
            let mut dir = false;
            let start = Instant::now();
            let mut count = 0;
            let mut max = calibration_pos;
            let mut min = calibration_pos;
            let mut s1 = Instant::now();
            while count < 100 {
                let pos = self.motor.read();
                max = max.max(pos);
                min = min.min(pos);
                if pos > calibration_pos {
                    if dir {
                        dir = false;
                        count += 1;
                    }
                    //trace!("bigger");
                    self.motor.set_current(-current).await;
                } else {
                    if !dir {
                        dir = true;
                        count += 1;
                    }
                    self.motor.set_current(current).await;
                }
                if s1.elapsed().as_micros() > 1000000 {
                    s1 = Instant::now();
                    //trace!("pos {}", pos);
                    Timer::after_millis(100).await;
                }
                Timer::after_micros(1).await;
                //trace!("pos {}", pos);
            }
            

            let duration = start.elapsed().as_micros();
            amplitude = max - min;
            //trace!("duration={} amplitude= {}", duration, amplitude);
            if (duration * 99) / 100 < previous_duration
                && previous_duration < (duration * 101) / 100
            {
                break;
            }
            previous_duration = duration;
        }

        self.motor.set_phase(0, 0.0);
        let k = current * 2.0 / (amplitude as f32) / f32::consts::FRAC_PI_4;
        let t = previous_duration as f32 / 100_000_000.0;
        use CalibrationMode::*;
        let (p, i, d) = match mode {
            P => (k / 2.0, 0.0, 0.0),
            PI => (k / 2.5, t / 1.25, 0.0),
            PID => (k * 0.6, t / 2.0, t / 8.0),
            Pessen => (k * 0.7, t * 0.4, t * 0.15),
            ModerateOvershoot => (k / 3.0, t / 2.0, t / 3.0),
            NoOvershoot => (k / 5.0, t / 2.0, t / 3.0),
        };
        self.pid.output_limit = 2.0;
        self.pid.p(p, 2.0);
        self.pid.i(i, 0.4);
        self.pid.d(d, 0.5);
        trace!("calibration done p={}, i={}, d={}", p, i, d);
        (p, i, d)
    }

    pub fn set_p(&mut self, gain: f32, limit: f32) {
        self.pid.p(gain, limit);
    }
    // TODO why is this an i32 and not an f32?
    pub fn set_objective(&mut self, obj: i32) {
        self.pid.setpoint(obj as f32);
    }
    pub async fn update(&mut self) {
        self.counter_ups += 1;
        let pos = self.motor.read();
        let out = self.pid.next_control_output(pos as f32).output;
        
        embassy_futures::yield_now().await;
        let diff = (self.pid.setpoint - pos as f32).abs();
        if diff > 20.0 {
            self.motor.set_current(out).await;
        } else if diff > 10.0 {
            self.motor.set_phase(
                (self.pid.setpoint as i32).rem_euclid(D::MICROSTEP as i32 * 4) as u8,
                1.8,
            );
        } else {
            self.motor.set_phase(0, 0.0);
        }
    }
}

#[cfg(test)]
mod tests {
    extern crate std;
    use crate::{EncoderTrait, std::get_fake_motor};
    use embassy_time::{Instant, Timer};
    use test_log::test;

    use super::PidController;
    #[test(tokio::test)]
    async fn test_pid() {
        let m = get_fake_motor();
        let mut pid = PidController::new(m, 2.0, 2.0);

        pid.calibration(2000, super::CalibrationMode::NoOvershoot)
            .await;

        for _ in 0..3 {
            pid.set_objective(10000);
            let t: Instant = Instant::now();
            while t.elapsed().as_millis() < 5000 {
                pid.update().await;
                Timer::after_micros(1).await;
            }
            assert!((pid.motor.read() - 10000).abs() < 20);
            let t = Instant::now();
            pid.set_objective(-10000);
            while t.elapsed().as_millis() < 5000 {
                pid.update().await;
                Timer::after_micros(1).await;
            }
            assert!((pid.motor.read() + 10000).abs() < 20);
        }
    }
}
