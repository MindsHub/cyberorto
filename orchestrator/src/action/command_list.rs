use std::{collections::VecDeque, fs::File, time::Duration};

use serde::{Deserialize, Serialize};

use crate::{state::StateHandler, util::serde::{deserialize_from_json_file, serialize_to_json_file}};

use super::{Action, Context};

#[derive(Debug, Serialize, Deserialize)]
pub struct CommandListAction {
    commands: VecDeque<Command>,
}

#[derive(Debug, Serialize, Deserialize)]
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

#[async_trait]
impl Action for CommandListAction {
    async fn step(&mut self, ctx: &Context, state_handler: &StateHandler) -> bool {
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

    fn get_type_name() -> &'static str where Self: Sized {
        "command_list"
    }

    fn save_to_disk(&self, ctx: &Context) -> Result<(), String> {
        serialize_to_json_file(&self, &ctx.get_save_dir().join("data.json"))
    }

    fn load_from_disk(ctx: &Context) -> Result<Self, String> where Self: Sized {
        deserialize_from_json_file(&ctx.get_save_dir().join("data.json"))
    }
}