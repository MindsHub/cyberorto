use pid::Pid;

use crate::{AsyncSerial, Comunication};

pub trait MotorAbstraction {
    // sign of current indicates if it is positive or negative
    fn set_phase(&mut self, current: f32);
    // in rotation?
    fn get_position(&mut self) -> f32;
}

pub struct PidMotor<M: MotorAbstraction> {
    motor: M,
    pid: Pid<f32>,
}
impl<M: MotorAbstraction> PidMotor<M> {
    fn new(motor: M) -> Self {
        Self {
            motor,
            // phase sphasament?
            pid: Pid::new(0.0f32, 10.0f32),
        }
    }
    fn update(&mut self) {
        let cur_pos = self.motor.get_position();
        let speed = self.pid.next_control_output(cur_pos).output;
        self.motor.set_phase(speed);
    }
}
/// rasp -> motore
/// gradi/mm

pub struct Motor<Serial: AsyncSerial, Motor: MotorAbstraction> {
    pub com: Comunication<Serial>,
    pub motor: Motor,
}

impl<Serial: AsyncSerial, Motore: MotorAbstraction> Motor<Serial, Motore> {
    fn update(&self) {}
}
#[cfg(all(test))] // , feature="std"
mod std {
    extern crate std;
    use super::*;
    use std::time::Instant;
    pub struct IdealMotor {
        last_update: Instant,
        rot_speed: f32,
        position: f32,
    }
    impl MotorAbstraction for IdealMotor {
        fn set_phase(&mut self, current: f32) {
            // 1A = 10rotation/s?
            self.rot_speed = current * 10.0;
        }

        fn get_position(&mut self) -> f32 {
            let elapsed = self.last_update.elapsed().as_secs_f32();
            self.position += elapsed * self.rot_speed;
            self.last_update = Instant::now();
            self.position
        }
    }
}
