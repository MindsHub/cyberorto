use crate::{AsyncSerial, Comunication, Sleep};

pub trait MotorAbstraction {
    fn set_obj(x: f32);
    fn reset();
    fn update();
}
pub struct Motor<Serial: AsyncSerial, Sleeper: Sleep> {
    pub com: Comunication<Serial, Sleeper>,
}
