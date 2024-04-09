use std::{collections::VecDeque, time::Duration};

use super::Action;

#[derive(Debug)]
pub struct CommandListAction {
    commands: VecDeque<Command>,
}

#[derive(Debug)]
pub enum Command {
    Move {
        x: f32,
        y: f32,
        z: f32,
    },
    Reset,
    Home,
    Retract,
    Water(Duration),
    Lights(Duration),
    AirPump(Duration),
    Plow(Duration),
}

impl Action for CommandListAction {
    fn step(&mut self, state_handler: &crate::state::StateHandler) -> bool {
        let command = if let Some(command) = self.commands.pop_front() {
            command
        } else {
            return false;
        };

        match command {
            Command::Move { x, y, z } => state_handler.move_to(x, y, z),
            Command::Reset => state_handler.reset(),
            Command::Home => state_handler.reset(), // TODO home()
            Command::Retract => state_handler.retract(),
            Command::Water(duration) => state_handler.water(duration),
            Command::Lights(duration) => state_handler.lights(duration),
            Command::AirPump(duration) => state_handler.air_pump(duration),
            Command::Plow(duration) => state_handler.plow(duration),
        }

        !self.commands.is_empty()
    }
}