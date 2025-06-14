pub trait DiscreteDriver {
    /// How many step each step is divided into?
    const MICROSTEP: usize;
    /// set current towards
    fn set_phase(&mut self, phase: u8, current: f32);
    fn get_microstep(&self) -> usize {
        Self::MICROSTEP
    }
}

pub trait EncoderTrait {
    fn read(&mut self) -> i32;
}
/*
pub trait Motor: DiscreteDriver + EncoderTrait {
    fn forward(&mut self, cur: f32);
    fn backward(&mut self, cur: f32);
    fn set_direction(&mut self, dir: bool);
    //fn here(&mut self, cur: f32);
}*/
/*
impl<T: DiscreteDriver + Encoder> Motor for T{
    fn forward(&mut self, cur: f32) {

    }

    fn backward(&mut self, cur: f32) {
        todo!()
    }
} */
