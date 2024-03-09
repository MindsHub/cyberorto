use std::fmt::Debug;

pub trait Action: Debug + Send {
    fn step(&mut self) -> bool;
}
