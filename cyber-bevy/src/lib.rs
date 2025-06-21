mod loading;
mod settings;

use std::{sync::mpsc, thread::{spawn, JoinHandle}};

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
)->(){

    
    /*
    mut loading_data: ResMut<LoadingData>,
    asset_server: ResMut<AssetServer>,
    let scene: Handle<Scene> =
        asset_server.load("embedded://cyber_bevy/embedded_assets/alessio.obj");
    loading_data.add_asset(&scene);
    let model: Handle<Image> = asset_server.load("embedded://cyber_bevy/embedded_assets/logo.png");
    loading_data.add_asset(&model);
    info!("setup");*/
    // add a circular base
    commands.spawn((
        Mesh3d(meshes.add(Circle::new(4.0))),
        MeshMaterial3d(materials.add(Color::srgb_u8(124, 144, 255))),
        Transform::from_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2))
            .with_translation(Vec3::new(0.0, -1.0, 0.0)),
        VisualizzationComponents,
        //Visibility::Hidden,
    ));
    //elementi orto
    //binario
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(5., 1., 0.4))),
        MeshMaterial3d(materials.add(Color::srgb_u8(171, 171, 170))),
        Transform::from_xyz(0., -0.5, 0.),
        VisualizzationComponents,
    ));
    //braccio
    let braccio = commands
        .spawn((
            Mesh3d(meshes.add(Cuboid::new(5., 0.25, 0.25))),
            MeshMaterial3d(materials.add(Color::srgb_u8(171, 171, 170))),
            Transform::from_xyz(0.75, -0.25, 0.),
            VisualizzationComponents,
        ))
        .id();

    let braccioz = commands
        .spawn((
            Mesh3d(meshes.add(Cuboid::new(0.25, 3., 0.25))),
            MeshMaterial3d(materials.add(Color::srgb_u8(171, 171, 170))),
            Transform::from_xyz(3.3, 0.2, 0.),
            Braccioz,
            VisualizzationComponents,
        ))
        .id();
    // torretta
    commands
        .spawn((
            Mesh3d(meshes.add(Cuboid::new(0.7, 2.5, 0.7))),
            MeshMaterial3d(materials.add(Color::srgb_u8(171, 171, 170))),
            Transform::from_xyz(0., 1.25, 0.),
            Torretta,
            VisualizzationComponents,
        ))
        .add_child(braccio)
        .add_child(braccioz);

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

fn muovi_torretta(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut torretta: Single<&mut Transform, With<Torretta>>,
    time: Res<Time>,
) {
    // A e D per muovere la torretta sul binario
    let segno = if keyboard_input.pressed(KeyCode::KeyD) {
        1.
    } else if keyboard_input.pressed(KeyCode::KeyA) {
        -1.
    } else {
        0.0
    };
    torretta.translation.x = (torretta.translation.x + time.delta_secs() * segno).clamp(-2.2, 2.2);

    let segno = if keyboard_input.pressed(KeyCode::ArrowLeft) {
        1.
    } else if keyboard_input.pressed(KeyCode::ArrowRight) {
        -1.
    } else {
        0.
    };
    torretta.rotate_y(time.delta_secs() * segno);
}

fn muovi_braccioz(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut braccioz: Single<&mut Transform, With<Braccioz>>,
    time: Res<Time>,
) {
    let segno = if keyboard_input.pressed(KeyCode::KeyW) {
        1.
    } else if keyboard_input.pressed(KeyCode::KeyS) {
        -1.
    } else {
        0.
    };
    braccioz.translation.y = (braccioz.translation.y + time.delta_secs() * segno).clamp(-0.6, 0.75);
}

pub fn spawn_bevy()->(JoinHandle<AppExit>, mpsc::Sender<Setter>) {
    let (sender, receiver) = mpsc::channel::<Setter>();
    let joinhandler = spawn(|| {
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
                (muovi_torretta, muovi_braccioz).run_if(in_state(LoadingState::Ready)),
            )
            .run()
    });
    (joinhandler, sender)
}
