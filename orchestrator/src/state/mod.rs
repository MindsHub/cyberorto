#![allow(unused_variables)] // TODO remove

pub(crate) mod tests;
pub mod dummy_message_handler;

use std::{
    sync::{Arc, Mutex, MutexGuard},
    time::Duration,
};

use embedcore::{prelude::*, protocol::cyber::Master};
use tokio_serial::SerialStream;

use crate::constants::ARM_LENGTH;
use crate::constants::WATER_TIME;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaterLevel {
    percentage: f32,
    liters: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatteryLevel {
    percentage: f32,
    volts: f32,
}

#[derive(Debug, Clone)]
pub struct State {
    // coordinates where the robots is going
    // TODO: convert to struct
    target_x: f32,
    target_y: f32,
    target_z: f32,

    // component flags
    water: bool,
    lights: bool,
    air_pump: bool,

    plow: bool,

    plants: Vec<Plant>,
    pub led_state: bool,

    // TODO: replace single vars with structs from api
    pub x: f32,
    pub y: f32,
    pub z: f32,

    pub battery_level: BatteryLevel,
    pub water_level: WaterLevel,
}

impl Default for State {
    fn default() -> Self {
        Self {
            target_x: 0.0,
            target_y: 0.0,
            target_z: 0.0,

            water: false,
            lights: false,
            air_pump: false,

            plow: false,
            led_state: false,
            plants: Vec::new(),

            x: 0.0,
            y: 0.0,
            z: 0.0,

            battery_level: BatteryLevel {
                percentage: 0.0,
                volts: 0.0,
            },
            water_level: WaterLevel {
                percentage: 0.0,
                liters: 0.0,
            },
        }
    }
}
#[derive(Debug, Clone)]
pub struct PlantTime {
    plant_timer: f32,
    water_timer: f32,
}

#[derive(Debug, Clone)]
pub struct Plant {
    x: f32,
    y: f32,
    z: f32,
}

#[derive(Debug, Clone)]
pub struct StateHandler {
    state: Arc<Mutex<State>>,
    master: Arc<Master<SerialStream>>,
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
    pub fn new(port: SerialStream) -> StateHandler {
        StateHandler {
            state: Arc::new(Mutex::new(State::default())),
            master: Arc::new(Master::new(port, 100000, 20)),
            //master: todo!(),
        }
    }

    pub fn get_state(&self) -> State {
        acquire(&self.state).clone()
    }

    pub fn water_a_plant(&self, x: f32, y: f32, z: f32) {
        self.move_to(x, y, z);
        self.water(Duration::from_secs_f32(WATER_TIME));
    }

    pub fn add_plant(&self, x: f32, y: f32, z: f32) {
        let mut state = acquire(&self.state);
        state.plants.push(Plant { x, y, z });
    }

    pub fn water_all(&self) {
        let plants = {
            let state = acquire(&self.state);
            state.plants.clone()
        };
        for plant in plants {
            self.water_a_plant(plant.x, plant.y, plant.z);
        }
    }

    pub fn water(&self, duration: Duration) {
        mutate_state!(&self.state, water = true);
        //self.master.water(duration);
        //self.master.water(duration);
        mutate_state!(&self.state, water = false);
    }
    pub async fn toggle_led(&self) {
        let s = !self.get_state().led_state;
        let _ = self.master.set_led(s).await;
        mutate_state!(&self.state, led_state = s)
    }

    pub fn lights(&self, duration: Duration) {
        mutate_state!(&self.state, lights = true);
        //self.master.lights(duration);
        mutate_state!(&self.state, lights = false);
    }

    pub fn air_pump(&self, duration: Duration) {
        mutate_state!(&self.state, air_pump = true);
        //self.master.pump(duration);
        mutate_state!(&self.state, air_pump = false);
    }

    pub fn plow(&self, duration: Duration) {
        mutate_state!(&self.state, plow = true);
        //self.master.plow(duration);
        mutate_state!(&self.state, plow = false);
    }

    pub fn home(&self) {
        //self.master.home();
        mutate_state!(&self.state, target_x = 0.0, target_y = 0.0, target_z = 0.0);
    }

    pub fn reset(&self) {
        //self.master.reset();
        mutate_state!(
            &self.state,
            target_x = 0.0,
            target_y = -ARM_LENGTH,
            target_z = 0.0
        );
    }

    pub fn retract(&self) {
        //self.master.retract();
        mutate_state!(&self.state, target_z = 0.0);
    }

    pub fn move_to(&self, x: f32, y: f32, z: f32) {
        //self.master.move_to(x).a;
        mutate_state!(&self.state, target_x = x, target_y = y, target_z = z);
    }
}
