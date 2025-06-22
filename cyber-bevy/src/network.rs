/// Code in this file was inspired by these bevy examples
/// https://bevy.org/examples/application/plugin/
/// https://gist.github.com/miketwenty1/baa1634fe558186e606c02932b8f37c8
use std::time::Duration;

use bevy::{app::{Plugin, Update}, ecs::{event::{EventReader, EventWriter, Events}, resource::Resource, schedule::IntoScheduleConfigs, system::{Res, ResMut}}, prelude::{Deref, DerefMut}, time::common_conditions::on_timer};
use bevy_http_client::{prelude::{HttpTypedRequestTrait, TypedRequest, TypedResponse, TypedResponseError}, HttpClient, HttpClientPlugin};
use definitions::RobotQueueState;

pub struct OrchestratorStateLoader {
    update_period: Duration,
    endpoint: String,
}

impl OrchestratorStateLoader {
    pub fn new(update_period: Duration, endpoint: String) -> Self {
        Self { update_period, endpoint }
    }
}


#[derive(Resource)]
struct OrchestratorStateLoaderRes {
    endpoint: String
}

#[derive(Resource, Deref, DerefMut, Default)]
pub struct OrchestratorStateOutput(RobotQueueState);

impl Plugin for OrchestratorStateLoader {
    fn build(&self, app: &mut bevy::app::App) {
        app
            .add_plugins(HttpClientPlugin)
            .insert_resource(OrchestratorStateLoaderRes { endpoint: self.endpoint.clone() })
            .insert_resource(OrchestratorStateOutput::default())
            .add_systems(Update, (handle_response, handle_error))
            .add_systems(Update, download_from_orchestrator_if_needed.run_if(on_timer(self.update_period)))
            .register_request_type::<RobotQueueState>();
    }
}

fn download_from_orchestrator_if_needed(
    res: Res<OrchestratorStateLoaderRes>,
    mut ev_request: EventWriter<TypedRequest<RobotQueueState>>,
) {
    ev_request.write(
        HttpClient::new()
            .get(format!("{}/state", res.endpoint))
            .with_type::<RobotQueueState>(),
    );
}

fn handle_response(mut events: ResMut<Events<TypedResponse<RobotQueueState>>>, mut state: ResMut<OrchestratorStateOutput>) {
    for response in events.drain() {
        let response: RobotQueueState = response.into_inner();
        //println!("got response: {response:?}");
        state.0 = response;
    }
}

fn handle_error(mut ev_error: EventReader<TypedResponseError<RobotQueueState>>) {
    for error in ev_error.read() {
        println!("Error retrieving {}", error.err);
    }
}

