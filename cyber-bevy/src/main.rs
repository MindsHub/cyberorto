mod loading;
mod settings;
mod network;

use std::time::Duration;

use bevy::{
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    prelude::*,
};
use bevy_asset::embedded_asset;
use bevy_egui::EguiPlugin;
use bevy_obj::ObjPlugin;
use bevy_panorbit_camera::{PanOrbitCamera, PanOrbitCameraPlugin};
use loading::{
    unload_current_visualization, LoadingScreenPlugin, LoadingState, VisualizzationComponents,
};
use settings::{Resolution, SettingsPlugin};

use crate::network::{OrchestratorStateOutput, OrchestratorStateLoader};

pub struct EmbeddedAssetPlugin;

impl Plugin for EmbeddedAssetPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "../embedded_assets/logo.png");
    }
}

pub enum Setter{
    X(f32),
    Y(f32),
    Z(f32),
}

pub fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let structure_color = materials.add(Color::srgb_u8(171, 171, 170));
    let terrain_color = materials.add(Color::srgb_u8(0x64, 0x61, 0x1C));

    /*
    mut loading_data: ResMut<LoadingData>,
    asset_server: ResMut<AssetServer>,
    let scene: Handle<Scene> =
        asset_server.load("embedded://cyber_bevy/embedded_assets/alessio.obj");
    loading_data.add_asset(&scene);
    let model: Handle<Image> = asset_server.load("embedded://cyber_bevy/embedded_assets/logo.png");
    loading_data.add_asset(&model);
    info!("setup");*/
    // terreno (rettangolo + 2 semicerchi)
    commands.spawn((
        // width and height will be scaled, so they need to be 1.0
        Mesh3d(meshes.add(Rectangle::new(1.0, 1.0))),
        MeshMaterial3d(terrain_color.clone()),
        Transform::from_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2))
            .with_translation(Vec3::new(0.0, -0.7, 0.0)),
        VisualizzationComponents,
        Terreno::Rettangolo,
    ));
    commands.spawn((
        // radius will be scaled, so it needs to be 0.5
        Mesh3d(meshes.add(Circle::new(0.5))),
        MeshMaterial3d(terrain_color.clone()),
        Transform::from_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2))
            .with_translation(Vec3::new(-0.5, -0.7, 0.0)),
        VisualizzationComponents,
        Terreno::Semicerchio { offset_wrt_rail_length: -0.5 },
    ));
    commands.spawn((
        // radius will be scaled, so it needs to be 0.5
        Mesh3d(meshes.add(Circle::new(0.5))),
        MeshMaterial3d(terrain_color.clone()),
        Transform::from_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2))
            .with_translation(Vec3::new(0.5, -0.7, 0.0)),
        VisualizzationComponents,
        Terreno::Semicerchio { offset_wrt_rail_length: 0.5 },
    ));
    //elementi orto
    //binario
    commands.spawn((
        // x will be scaled, so it needs to be 1.0
        Mesh3d(meshes.add(Cuboid::new(1.0, 0.7, 0.05))),
        MeshMaterial3d(structure_color.clone()),
        Transform::from_xyz(0., -0.35, 0.),
        VisualizzationComponents,
        Binario,
    ));
    //braccio
    let braccio = commands
        .spawn((
            // x will be scaled, so it needs to be 1.0
            Mesh3d(meshes.add(Cuboid::new(1.0, 0.05, 0.05))),
            MeshMaterial3d(structure_color.clone()),
            Transform::from_xyz(-0.5, -0.2, 0.),
            VisualizzationComponents,
            Braccio,
        ))
        .id();
    let braccio_retro = commands
        .spawn((
            // x will be scaled, so it needs to be 1.0
            Mesh3d(meshes.add(Cuboid::new(0.8, 0.05, 0.05))),
            MeshMaterial3d(structure_color.clone()),
            Transform::from_xyz(0.4, -0.2, 0.),
            VisualizzationComponents,
        ))
        .id();

    let braccioz = commands
        .spawn((
            Mesh3d(meshes.add(Cuboid::new(0.05, 1.6, 0.05))),
            MeshMaterial3d(structure_color.clone()),
            Transform::from_xyz(-1.0, 0.4, 0.),
            Braccioz,
            VisualizzationComponents,
        ))
        .id();
    // torretta
    commands
        .spawn((
            Mesh3d(meshes.add(Cylinder::new(0.16, 1.0))),
            MeshMaterial3d(structure_color.clone()),
            Transform::from_xyz(0., 0.5, 0.),
            Torretta,
            VisualizzationComponents,
        ))
        .add_child(braccio)
        .add_child(braccio_retro)
        .add_child(braccioz);

    //arrivo
    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(0.05))),
        MeshMaterial3d(materials.add(Color::srgb(2.0,0.0,0.0))),
        Arrivo,
        VisualizzationComponents,
    ));

    // luce
    commands.spawn((
        PointLight {
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(4., 8., 4.),
        VisualizzationComponents,
    ));
    // camera
    commands.spawn((
        Camera3d::default(),
        Camera {
            is_active: false,
            ..default()
        },
        Transform::from_xyz(-2.5, 4.5, 9.).looking_at(Vec3::ZERO, Vec3::Y),
        PanOrbitCamera {
            pitch_lower_limit: Some(0.0),
            ..default()
        },
        VisualizzationComponents,
    ));
}

#[derive(Component)]
struct Torretta;

#[derive(Component)]
struct Braccioz;

#[derive(Component)]
struct Binario;

#[derive(Component)]
struct Braccio;

#[derive(Component)]
struct Arrivo;

#[derive(Component)]
enum Terreno {
    Rettangolo,
    Semicerchio {
        /// Il terreno è fatto di tre figure e questa variabile dice la
        /// posizione relativa rispetto alla rotaia.
        offset_wrt_rail_length: f32,
    }
}

fn muovi_torretta(
    mut torretta: Single<&mut Transform, With<Torretta>>,
    state: Res<OrchestratorStateOutput>,
) {
    torretta.translation.x = state.position_joint.x - state.parameters.rail_length / 2.0;
    torretta.rotation = Quat::from_rotation_y(state.position_joint.y);
}

fn muovi_braccioz(
    mut braccioz: Single<&mut Transform, With<Braccioz>>,
    state: Res<OrchestratorStateOutput>,
) {
    braccioz.translation.y = state.position_joint.z + 0.4;
    braccioz.translation.x = -state.parameters.arm_length;
}

fn muovi_arrivo(
    mut arrivo: Single<&mut Transform, With<Arrivo>>,
    state: Res<OrchestratorStateOutput>,
) {
    arrivo.translation.y = -state.target.y;
    arrivo.translation.x = state.target.x-(state.parameters.rail_length / 2.0);
    arrivo.translation.z = state.target.z;
}

fn cambia_rotaia(
    mut binario: Single<&mut Transform, With<Binario>>,
    state: Res<OrchestratorStateOutput>,
) {
    binario.scale.x = state.parameters.rail_length;
}

fn cambia_braccio(
    mut braccio: Single<&mut Transform, With<Braccio>>,
    state: Res<OrchestratorStateOutput>,
) {
    braccio.translation.x = -state.parameters.arm_length / 2.0;
    braccio.scale.x = state.parameters.arm_length;
}

fn cambia_terreni(
    mut terreni: Query<(&mut Transform, &Terreno)>,
    state: Res<OrchestratorStateOutput>,
) {
    for mut terreno in &mut terreni {
        terreno.0.scale.y = state.parameters.arm_length * 2.0;
        match terreno.1 {
            Terreno::Rettangolo => {
                terreno.0.scale.x = state.parameters.rail_length;
            },
            Terreno::Semicerchio { offset_wrt_rail_length } => {
                terreno.0.scale.x = state.parameters.arm_length * 2.0;
                terreno.0.translation.x = state.parameters.rail_length * offset_wrt_rail_length;
            },
        }
    }
}

pub fn spawn_bevy() -> AppExit {
    let window = WindowPlugin {
        primary_window: Some(Window {
            title: "Cyber Bevy".to_string(),
            ..default()
        }),
        ..default()
    };
    App::new()
        // default plugin
        .add_plugins(DefaultPlugins.set(window))
        // libraries plugins
        .add_plugins((
            FrameTimeDiagnosticsPlugin::default(),
            LogDiagnosticsPlugin::default(),
        ))
        .add_plugins(ObjPlugin)
        .add_plugins(EguiPlugin {
            enable_multipass_for_primary_context: true,
        })
        .add_plugins(PanOrbitCameraPlugin)
        // custom plugins
        .add_plugins(EmbeddedAssetPlugin)
        .add_plugins(SettingsPlugin)
        .add_plugins(LoadingScreenPlugin {
            img_path: "embedded://cyber_bevy/embedded_assets/logo.png".to_string(),
        })
        .add_plugins(OrchestratorStateLoader::new(Duration::from_millis(100), "http://127.0.0.1:8000".to_string()))
        // insert setup function
        .insert_resource(ClearColor(Color::srgb(0.231, 0.31, 0.271)))
        .add_systems(
            Update,
            (unload_current_visualization, setup)
                .chain()
                .run_if(resource_changed::<Resolution>),
        )
        .add_systems(
            Update,
            (muovi_torretta, muovi_braccioz, muovi_arrivo, cambia_rotaia, cambia_braccio, cambia_terreni)
                .run_if(resource_changed::<OrchestratorStateOutput>)
                .run_if(in_state(LoadingState::Ready)),
        )
        .run()
}

fn main() -> AppExit {
    spawn_bevy()
}

