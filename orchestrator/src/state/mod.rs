#![allow(unused_variables)] // TODO remove

pub(crate) mod tests;
pub mod dummy_message_handler;

use std::{
    sync::{Arc, Mutex, MutexGuard}, time::Duration
};

use definitions::RobotState;
use embedcore::protocol::cyber::Master;
use rocket::futures::future;
use tokio_serial::SerialStream;

use crate::constants::{ARM_LENGTH, WATER_TIME_MS};

#[derive(Debug, Clone)]
pub struct Plant {
    x: f32,
    y: f32,
    z: f32,
}

type State = RobotState;

/// TODO decide whether to remove the `state` field completely, and rather ask for state directly
/// to the connected devices every time
#[derive(Debug, Clone)]
pub struct StateHandler {
    state: Arc<Mutex<State>>,
    master_x: Arc<Master<SerialStream>>,
    master_y: Arc<Master<SerialStream>>,
    master_z: Arc<Master<SerialStream>>,
    /// Sensors might be implemented by a motor, so this may be a clone of one of
    /// master_x, master_y, master_z, so avoid using it while also using a motor!
    master_peripherals: Arc<Master<SerialStream>>,
}

fn acquire(state: &Arc<Mutex<State>>) -> MutexGuard<'_, State> {
    match state.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

macro_rules! mutate_state {
    ($state:expr, $($($field:ident).+ = $value:expr),+ $(,)?) => {
        {
            let mut state = acquire($state);
            $(state$(.$field)+ = $value;)*
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
            master_peripherals: master.clone(),
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
        // TODO add plant to DB
    }

    pub async fn water_all(&self) -> Result<(), ()> {
        let plants: Vec<Plant> = vec![]; // TODO load plant from DB
        for plant in plants {
            self.water_a_plant(plant.x, plant.y, plant.z).await?;
        }
        Ok(())
    }

    pub async fn water(&self, cooldown_ms: u64) -> Result<(), ()> {
        self.master_peripherals.water(cooldown_ms).await?;
        // TODO remove and query state elsewhere, the above does not wait for completion!
        //a mutate_state!(&self.state, water = cooldown_ms != 0);
        Ok(())
    }

    pub async fn lights(&self, cooldown_ms: u64) -> Result<(), ()> {
        self.master_peripherals.lights(cooldown_ms).await?;
        // TODO remove and query state elsewhere, the above does not wait for completion!
        //a mutate_state!(&self.state, lights = cooldown_ms != 0);
        Ok(())
    }

    pub async fn pump(&self, cooldown_ms: u64) -> Result<(), ()> {
        self.master_peripherals.pump(cooldown_ms).await?;
        Ok(())
    }

    pub async fn plow(&self, cooldown_ms: u64) -> Result<(), ()> {
        self.master_peripherals.plow(cooldown_ms).await?;
        Ok(())
    }

    pub async fn toggle_led(&self) -> Result<(), ()> {
        let curr_led = self.master_peripherals.get_state().await?.led;
        self.master_peripherals.set_led(!curr_led).await?;
        Ok(())
    }

    pub async fn home(&self) -> Result<(), ()> {
        self.move_to(0.0, 0.0, 0.0).await
    }

    pub async fn reset(&self) -> Result<(), ()> {
        // first wait for the Z axis to reset
        self.retract().await?;

        // then also reset X and Y in parallel (to make things faster)
        mutate_state!(&self.state, target.x = 0.0, target.y = -ARM_LENGTH);
        let (res_x, res_y) = future::join(
            self.master_x.reset(),
            self.master_y.reset(),
        ).await;
        res_x?;
        res_y?;
        mutate_state!(&self.state, position.x = 0.0, position.y = -ARM_LENGTH);

        Ok(())
    }

    pub async fn retract(&self) -> Result<(), ()> {
        // "retract" means resetting just the Z axis
        mutate_state!(&self.state, target.z = 0.0);
        self.master_z.reset().await?;
        mutate_state!(&self.state, position.z = 0.0);
        Ok(())
    }

    pub async fn move_to(&self, x: f32, y: f32, z: f32) -> Result<(), ()> {
        // TODO compute trajectory that avoids obstacles and optimizes path
        mutate_state!(&self.state, target.x = x, target.y = y, target.z = z);
        self.master_x.move_to(x).await?;
        self.master_y.move_to(y).await?;
        self.master_z.move_to(z).await?;
        mutate_state!(&self.state, position.x = x, position.y = y, position.z = z);
        Ok(())
    }

    pub async fn update_state(&self) -> Result<(), ()> {
        let (x, y, z, perhipherals) = rocket::futures::future::join4(
            self.master_x.get_state(),
            self.master_y.get_state(),
            self.master_z.get_state(),
            self.master_peripherals.get_state()
        ).await;

        let x = x?;
        let y = y?;
        let z = z?;
        let perhipherals = perhipherals?;

        mutate_state!(
            &self.state,
            position.x = x.motor_pos,
            position.y = y.motor_pos,
            position.z = z.motor_pos,
            actuators.water = perhipherals.water,
            actuators.lights = perhipherals.lights,
            actuators.pump = perhipherals.pump,
            actuators.plow = perhipherals.plow,
            actuators.led = perhipherals.led,
        );

        Ok(())
    }
}
