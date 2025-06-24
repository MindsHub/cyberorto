#![allow(unused_variables)] // TODO remove

pub(crate) mod tests;
pub mod dummy_message_handler;
mod kinematics;

use std::{
    sync::{Arc, Mutex, MutexGuard}, time::Duration
};

use definitions::{Parameters, RobotState, Vec3};
use embedcore::protocol::cyber::Master;
use rocket::futures::future::{self, join4};
use tokio_serial::SerialStream;

use crate::{constants::{ARM_LENGTH, WATER_TIME_MS}, state::kinematics::{joint_to_world, world_to_joint}};

#[derive(Debug, Clone)]
pub struct Plant {
    x: f32,
    y: f32,
    z: f32,
}

type State = RobotState;

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
            state: Arc::new(Mutex::new(State {
                // TODO read parameters from file
                parameters: Parameters {
                    arm_length: 1.511, // meters
                    rail_length: 5.3, // meters
                },
                ..Default::default()
            })),
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
        Ok(())
    }

    pub async fn lights(&self, cooldown_ms: u64) -> Result<(), ()> {
        self.master_peripherals.lights(cooldown_ms).await?;
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
        let params = self.get_state().parameters.clone();
        let world = Vec3 { x, y, z };
        let Some(joint) = world_to_joint(&world, &params) else {
            return Err(());
        };

        // TODO compute trajectory that avoids obstacles and optimizes path
        // TODO handle errors while motors are moving and stop everything if errors happen
        mutate_state!(&self.state, target = world, target_joint = joint.clone());
        self.master_x.move_to(joint.x).await?;
        self.master_y.move_to(joint.y).await?;
        self.master_z.move_to(joint.z).await?;
        Ok(())
    }

    pub async fn try_update_state(&self) -> State {
        let (x, y, z, peripherals) = join4(
            self.master_x.get_state(),
            self.master_y.get_state(),
            self.master_z.get_state(),
            self.master_peripherals.get_state()
        ).await;

        let mut state = acquire(&self.state);

        match x {
            Ok(x) => state.position_joint.x = x.motor_pos,
            Err(_) => state.errors.motor_x = true,
        }
        match y {
            Ok(y) => state.position_joint.y = y.motor_pos,
            Err(_) => state.errors.motor_y = true,
        }
        match z {
            Ok(z) => state.position_joint.z = z.motor_pos,
            Err(_) => state.errors.motor_z = true,
        }
        state.position = joint_to_world(&state.position_joint, &state.parameters);

        match peripherals {
            Ok(peripherals) => {
                state.actuators.water = peripherals.water;
                state.actuators.lights = peripherals.lights;
                state.actuators.pump = peripherals.pump;
                state.actuators.plow = peripherals.plow;
                state.actuators.led = peripherals.led;
            }
            Err(_) => state.errors.peripherals = true,
        }

        state.clone()
    }
}
