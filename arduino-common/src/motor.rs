use crate::{AsyncSerial, Comunication, Sleep};

pub trait MotorAbstraction {
    fn set_phase(&mut self, phase: f32, current: f32);
    fn get_position(&mut self) -> f32;
}
/// rasp -> motore
/// gradi/mm

pub struct Motor<Serial: AsyncSerial, Sleeper: Sleep, Motor: MotorAbstraction> {
    pub com: Comunication<Serial, Sleeper>,
    pub motor: Motor,
}

//impl<Serial: AsyncSerial, Sleeper: Sleep, Motor: MotorAbstraction> Motor<Serial, Sleep<>>
