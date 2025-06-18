#![allow(unused_variables)] // TODO remove

pub(crate) mod tests;
pub mod dummy_message_handler;

use std::{
    sync::{Arc, Mutex, MutexGuard}, time::Duration
};

use embedcore::protocol::cyber::Master;
use rocket::futures::future;
use tokio_serial::SerialStream;

use crate::constants::{ARM_LENGTH, WATER_TIME_MS};
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
    pub target_x: f32,
    pub target_y: f32,
    pub target_z: f32,

    // component flags
    pub water: bool,
    pub lights: bool,
    pub air_pump: bool,
    pub plow: bool,
    pub led_state: bool,

    // TODO take plants from a database
    plants: Vec<Plant>,

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
pub struct Plant {
    x: f32,
    y: f32,
    z: f32,
}

#[derive(Debug, Clone)]
pub struct StateHandler {
    state: Arc<Mutex<State>>,
    master_x: Arc<Master<SerialStream>>,
    master_y: Arc<Master<SerialStream>>,
    master_z: Arc<Master<SerialStream>>,
    /// Sensors might be implemented by a motor, so this may be a clone of one of
    /// master_x, master_y, master_z, so avoid using it while also using a motor!
    master_sensors: Arc<Master<SerialStream>>,
}

fn acquire(state: &Arc<Mutex<State>>) -> MutexGuard<'_, State> {
    match state.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

macro_rules! mutate_state {
    ($state:expr, $($field:ident = $value:expr),+ $(,)?) => {
        {
            let mut state = acquire($state);
            $(state.$field = $value;)*
        }
    };
}

impl StateHandler {
    pub fn new(port: SerialStream) -> StateHandler {
        // TODO use different masters for X, Y, Z and sensors (detect it from their name/capabilities)
        let master= Arc::new(Master::new(port, 100000, 20));
        StateHandler {
            state: Arc::new(Mutex::new(State::default())),
            master_x: master.clone(),
            master_y: master.clone(),
            master_z: master.clone(),
            master_sensors: master.clone(),
        }
    }

    pub fn get_state(&self) -> State {
        acquire(&self.state).clone()
    }

    pub async fn water_a_plant(&self, x: f32, y: f32, z: f32) -> Result<(), ()> {
        self.move_to(x, y, z).await?;
        self.water(WATER_TIME_MS).await?;
        tokio::time::sleep(Duration::from_millis(WATER_TIME_MS)).await;
        self.water(0).await?;
        Ok(())
    }

    pub fn add_plant(&self, x: f32, y: f32, z: f32) {
        let mut state = acquire(&self.state);
        state.plants.push(Plant { x, y, z });
    }

    pub async fn water_all(&self) -> Result<(), ()> {
        let plants = {
            let state = acquire(&self.state);
            state.plants.clone()
        };
        for plant in plants {
            self.water_a_plant(plant.x, plant.y, plant.z).await?;
        }
        Ok(())
    }

    pub async fn water(&self, cooldown_ms: u64) -> Result<(), ()> {
        self.master_sensors.water(cooldown_ms).await?;
        // TODO remove and query state elsewhere, the above does not wait for completion!
        mutate_state!(&self.state, water = cooldown_ms != 0);
        Ok(())
    }

    pub async fn lights(&self, cooldown_ms: u64) -> Result<(), ()> {
        self.master_sensors.lights(cooldown_ms).await?;
        // TODO remove and query state elsewhere, the above does not wait for completion!
        mutate_state!(&self.state, lights = cooldown_ms != 0);
        Ok(())
    }

    pub async fn pump(&self, cooldown_ms: u64) -> Result<(), ()> {
        self.master_sensors.pump(cooldown_ms).await?;
        // TODO remove and query state elsewhere, the above does not wait for completion!
        mutate_state!(&self.state, air_pump = cooldown_ms != 0);
        Ok(())
    }

    pub async fn plow(&self, cooldown_ms: u64) -> Result<(), ()> {
        self.master_sensors.plow(cooldown_ms).await?;
        // TODO remove and query state elsewhere, the above does not wait for completion!
        mutate_state!(&self.state, plow = cooldown_ms != 0);
        Ok(())
    }

    pub async fn toggle_led(&self) -> Result<(), ()> {
        let s = !self.get_state().led_state;
        self.master_sensors.set_led(s).await?;
        mutate_state!(&self.state, led_state = s);
        Ok(())
    }

    pub async fn home(&self) -> Result<(), ()> {
        self.move_to(0.0, 0.0, 0.0).await
    }

    pub async fn reset(&self) -> Result<(), ()> {
        // first wait for the Z axis to reset
        self.retract().await?;

        // then also reset X and Y in parallel (to make things faster)
        mutate_state!(&self.state, target_x = 0.0, target_y = -ARM_LENGTH);
        let (res_x, res_y) = future::join(
            self.master_x.reset(),
            self.master_y.reset(),
        ).await;
        res_x?;
        res_y?;
        mutate_state!(&self.state, x = 0.0, y = -ARM_LENGTH);

        Ok(())
    }

    pub async fn retract(&self) -> Result<(), ()> {
        // "retract" means resetting just the Z axis
        mutate_state!(&self.state, target_z = 0.0);
        self.master_z.reset().await?;
        mutate_state!(&self.state, z = 0.0);
        Ok(())
    }

    pub async fn move_to(&self, x: f32, y: f32, z: f32) -> Result<(), ()> {
        // TODO compute trajectory that avoids obstacles and optimizes path
        mutate_state!(&self.state, target_x = x, target_y = y, target_z = z);
        self.master_x.move_to(x).await?;
        self.master_y.move_to(y).await?;
        self.master_z.move_to(z).await?;
        mutate_state!(&self.state, x = x, y = y, z = z);
        Ok(())
    }
}
