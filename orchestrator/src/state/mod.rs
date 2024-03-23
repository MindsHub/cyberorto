use std::{
    sync::{Arc, Mutex, MutexGuard},
    thread,
    time::Duration, vec,
};

use crate::constants::ARM_LENGTH;
use crate::constants::WATER_TIME;

#[derive(Debug)]
pub struct State {
    target_x: f64,
    target_y: f64,
    target_z: f64,
    x: f64,
    y: f64,
    z: f64,
    water: bool,
    lights: bool,
    air_pump: bool,
    plow: bool,
    plants: Vec<Plant>,
    
}

impl Default for State {
    fn default() -> Self {
        Self {
            target_x: 0.0,
            target_y: 0.0,
            target_z: 0.0,
            x: 0.0,
            y: 0.0,
            z: 0.0,
            water: false,
            lights: false,
            air_pump: false,
            plow: false,
            plants: Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Plant{
    x: f64,
    y: f64,
    z: f64,
}

#[derive(Debug, Clone)]
pub struct StateHandler {
    state: Arc<Mutex<State>>,

    // TODO add serial object
}

fn acquire(state: &Arc<Mutex<State>>) -> MutexGuard<'_, State> {
    match state.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

macro_rules! mutate_state {
    ($state:expr, $($field:ident = $value:expr),+) => {
        {
            let mut state = acquire($state);
            $(state.$field = $value;)*
        }
    };
}

impl StateHandler {
    pub fn new() -> StateHandler {
        StateHandler {
            state: Arc::new(Mutex::new(State::default())),
        }
    }

    pub fn water_a_plant(&self, x: f64, y: f64, z: f64) {
        self.move_to(x, y, z);
        self.water(Duration::from_secs_f64(WATER_TIME));
    }

    pub fn add_plant(&self, x: f64, y: f64, z: f64) {
        let mut state = acquire(&self.state);
        state.plants.push(Plant{x:x, y:y, z:z});
    }

    pub fn water_all(&self) {
        let plants = {
            let mut state = acquire(&self.state);
            state.plants.clone()
        };
        for plant in plants {
            self.water_a_plant(plant.x, plant.y, plant.z);
        }
        
    }

    pub fn autopilot(&self) {


    }




    



    
    pub fn water(&self, duration: Duration) {
        // TODO send command to Arduino to turn on water
        mutate_state!(&self.state, water = true);
        thread::sleep(duration);
        // TODO send command to Arduino to turn off water
        mutate_state!(&self.state, water = false);
    }

    pub fn lights(&self, duration: Duration) {
        // TODO send command to Arduino to turn on the lights
        mutate_state!(&self.state, lights = true);
        thread::sleep(duration);
        // TODO send command to Arduino to turn off the lights
        mutate_state!(&self.state, lights = false);
    }

    pub fn air_pump(&self, duration: Duration) {
        // TODO send command to Arduino to turn on the air pump
        mutate_state!(&self.state, air_pump = true);
        thread::sleep(duration);
        // TODO send command to Arduino to turn off the air pump
        mutate_state!(&self.state, air_pump = false);
    }

    pub fn plow(&self, duration: Duration) {
        // TODO send command to Arduino to turn on the air pump
        mutate_state!(&self.state, plow = true);
        thread::sleep(duration);
        // TODO send command to Arduino to turn off the air pump
        mutate_state!(&self.state, plow = false);
    }

    pub fn reset(&self) {
        // TODO send command to Arduino
        mutate_state!(&self.state, target_x = 0.0, target_y = -ARM_LENGTH, target_z = 0.0);
    }

    pub fn retract(&self) {
        // TODO send command to Arduino
        mutate_state!(&self.state, target_z = 0.0);
    }

    pub fn move_to(&self, x: f64, y: f64, z: f64) {
        // TODO send command to Arduino
        mutate_state!(&self.state, target_x = x, target_y = y, target_z = z);
    }
}
