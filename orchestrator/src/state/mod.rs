use std::{
    sync::{Arc, Mutex, MutexGuard},
    thread,
    time::Duration,
};

use crate::constants::ARM_LENGTH;

#[derive(Debug)]
pub struct State {
    target_x: f64,
    target_y: f64,
    target_z: f64,
    x: f64,
    y: f64,
    z: f64,
    water: bool,
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
        }
    }
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

    pub fn move_to(&self, x: f64, y: f64, z: f64) {
        // TODO send command to Arduino
        mutate_state!(&self.state, target_x = x, target_y = y, target_z = z);
    }

    pub fn water(&self, duration: Duration) {
        // TODO send command to Arduino to turn on water
        mutate_state!(&self.state, water = true);
        thread::sleep(duration);
        // TODO send command to Arduino to turn off water
        mutate_state!(&self.state, water = false);
    }

    pub fn reset(&self) {
        // TODO send command to Arduino
        mutate_state!(
            &self.state,
            target_x = 0.0,
            target_y = -ARM_LENGTH,
            target_z = 0.0
        );
    }

    pub fn retract(&self) {
        // TODO send command to Arduino
        mutate_state!(&self.state, target_z = 0.0);
    }
}
