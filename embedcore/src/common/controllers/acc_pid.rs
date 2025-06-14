/*
use crate::{DiscreteDriver, EncoderTrait};

use super::pid::PidController;
pub struct AccPid<E: EncoderTrait, D: DiscreteDriver>{
    pub pid: PidController<E, D>,
    acceleration: f32,
    objective: i32,
    cur_speed: f32,
    max_speed: f32,
    
}
impl<E: EncoderTrait, D: DiscreteDriver> AccPid<E, D>{
    pub fn new(mut pid: PidController<E, D>, acceleration: f32, max_speed: f32) -> Self{
        pid.set_objective(0);
        /*Self{
            pid,
            cur_pos: 0.0,
            acceleration,
            max_speed,
            objective: 0.0,
        }*/
        todo!()
    }
    pub fn set_objective(&mut self, objective: f32){
        self.objective = todo!();
    }
    pub async fn update(&mut self){
        //let next;
        // if enough space to decelerate, accelerate (until saturation)
        // else decelerate





        self.pid.set_objective(todo!());
        self.pid.update().await;
    }
}

    */