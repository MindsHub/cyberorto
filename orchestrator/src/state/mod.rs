use std::{
    cell::RefCell, future::Future, sync::{Arc, Mutex, MutexGuard}, thread, time::Duration, vec
};

use arduino_common::prelude::*;

use crate::constants::ARM_LENGTH;
use crate::constants::WATER_TIME;
use serde::{Deserialize, Serialize};


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaterLevel {
    percentage: f32,
    liters:     f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatteryLevel {
    percentage: f32,
    volts:      f32,
}


#[derive(Debug, Clone)]
pub struct State {
    // coordinates where the robots is going
    // TODO: convert to struct
    target_x: f32,
    target_y: f32,
    target_z: f32,

    // component flags
    water:    bool,
    lights:   bool,
    air_pump: bool,

    plow: bool,

    plants: Vec<Plant>,

    // TODO: replace single vars with structs from api
    pub x: f32,
    pub y: f32,
    pub z: f32,

    pub battery_level: BatteryLevel,
    pub water_level:   WaterLevel,
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

            plants: Vec::new(),

            x: 0.0,
            y: 0.0,
            z: 0.0,

            battery_level: BatteryLevel {
                percentage: 0.0,
                volts:      0.0,
            },
            water_level: WaterLevel {
                percentage: 0.0,
                liters:     0.0,
            }
        }
    }
}
#[derive(Debug, Clone)]
pub struct PlantTime{
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
    master: Arc<Master<Plant, Plant, tokio::sync::Mutex<InnerMaster<Plant, Plant>>>>,
    // TODO add serial object
}

impl AsyncSerial for Plant {
    async fn read(&mut self) -> u8{
        todo!()
    }

    async fn write(&mut self, buf: u8) {
        todo!()
    }
}

impl Sleep for Plant {
    fn await_us(us: u64) -> Self {
        todo!()
    }
}
impl Future for Plant{
    type Output=();

    fn poll(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Self::Output> {
        todo!()
    }
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
            master: Arc::new(Master::new(todo!(), 100, 20)),
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
        state.plants.push(Plant { x: x, y: y, z: z });
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

    pub fn water(&self, duration: Duration) {
        mutate_state!(&self.state, water = true);
        //self.master.water(duration);
        mutate_state!(&self.state, water = false);
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
        mutate_state!(&self.state, target_x = 0.0, target_y = -ARM_LENGTH, target_z = 0.0);
    }

    pub fn retract(&self) {
        //self.master.retract();
        mutate_state!(&self.state, target_z = 0.0);
    }

    pub fn move_to(&self, x: f32, y: f32, z: f32) {
        self.master.move_to(x, y, z);
        mutate_state!(&self.state, target_x = x, target_y = y, target_z = z);
    }
}
