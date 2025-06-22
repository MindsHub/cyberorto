/// Code in this file was inspired by these bevy examples
/// https://bevy.org/examples/application/plugin/
/// https://gist.github.com/miketwenty1/baa1634fe558186e606c02932b8f37c8
use std::{path::PathBuf, time::Duration};

use bevy::{app::{Plugin, Update}, ecs::{event::{EventReader, EventWriter, Events}, resource::Resource, schedule::IntoScheduleConfigs, system::{Res, ResMut}}, time::common_conditions::on_timer};
use bevy_http_client::{prelude::{HttpTypedRequestTrait, TypedRequest, TypedResponse, TypedResponseError}, HttpClient, HttpClientPlugin};
use serde::{Deserialize, Serialize};

pub struct OrchestratorStateLoader {
    update_period: Duration,
    endpoint: String,
}

#[derive(Resource)]
struct OrchestratorStateLoaderRes {
    endpoint: String
}


#[derive(Serialize, Deserialize, Default, Resource, Clone, Debug)]
pub struct OrchestratorState {
    pub position: Position,
    pub target: Position,
    pub water_level: WaterLevel,
    pub battery_level: BatteryLevel,
    pub queue: QueueState,
    pub devices: Devices,
}

impl OrchestratorStateLoader {
    pub fn new(update_period: Duration, endpoint: String) -> Self {
        Self { update_period, endpoint }
    }
}

impl Plugin for OrchestratorStateLoader {
    fn build(&self, app: &mut bevy::app::App) {
        app
            .add_plugins(HttpClientPlugin)
            .insert_resource(OrchestratorStateLoaderRes { endpoint: self.endpoint.clone() })
            .insert_resource(OrchestratorState::default())
            .add_systems(Update, (handle_response, handle_error))
            .add_systems(Update, download_from_orchestrator_if_needed.run_if(on_timer(self.update_period)))
            .register_request_type::<OrchestratorState>();
    }
}

fn download_from_orchestrator_if_needed(
    res: Res<OrchestratorStateLoaderRes>,
    mut ev_request: EventWriter<TypedRequest<OrchestratorState>>,
) {
    ev_request.write(
        HttpClient::new()
            .get(format!("{}/state", res.endpoint))
            .with_type::<OrchestratorState>(),
    );
}

fn handle_response(mut events: ResMut<Events<TypedResponse<OrchestratorState>>>, mut state: ResMut<OrchestratorState>) {
    for response in events.drain() {
        let response: OrchestratorState = response.into_inner();
        //println!("got response: {response:?}");
        *state = response;
    }
}

fn handle_error(mut ev_error: EventReader<TypedResponseError<OrchestratorState>>) {
    for error in ev_error.read() {
        println!("Error retrieving {}", error.err);
    }
}

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct Position {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct Devices {
    pub water: bool,
    pub lights: bool,
    pub pump: bool,
    pub plow: bool,
    pub led: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WaterLevel {
    pub percentage: f32,
    pub liters: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BatteryLevel {
    pub percentage: f32,
    pub volts: f32,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct QueueState {
    pub paused: bool,
    pub stopped: bool,
    pub emergency: EmergencyStatus,
    pub save_dir: PathBuf,
    pub running_id: Option<ActionId>,
    pub actions: Vec<ActionInfo>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone, Copy, Default)]
pub enum EmergencyStatus {
    #[default]
    None,
    WaitingForReset,
    Resetting,
}

pub type ActionId = u32;

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct ActionInfo {
    id: ActionId,
    type_name: String,
    save_dir: PathBuf,
    is_running: bool,
}
