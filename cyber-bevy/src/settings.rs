use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};

#[derive(Default, Debug, Resource, PartialEq, Eq, Clone)]
pub enum Resolution {
    #[default]
    Cube,
    Low,
    Medium,
    High,
}

#[derive(Debug, Default, PartialEq, Eq, States, Hash, Clone)]
pub enum SettingState {
    Open,
    #[default]
    Closed,
}

pub fn ui_example_system(mut contexts: EguiContexts, mut resolution: ResMut<Resolution>) {
    egui::Window::new("Settings").show(contexts.ctx_mut(), |ui| {
        let start = resolution.clone();
        let mut cur = resolution.clone();
        egui::ComboBox::from_label("Select one!")
            .selected_text(format!("{:?}", resolution.as_ref()))
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut cur, Resolution::Cube, "Cube");
                ui.selectable_value(&mut cur, Resolution::Low, "Low");
                ui.selectable_value(&mut cur, Resolution::Medium, "Medium");
                ui.selectable_value(&mut cur, Resolution::High, "High");
            });
        if start != cur {
            *resolution = cur;
        }
    });
}

pub struct SettingsPlugin;

impl Plugin for SettingsPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<SettingState>()
            .init_resource::<Resolution>()
            .add_systems(
                Update,
                ui_example_system.run_if(in_state(SettingState::Open)),
            )
            .add_systems(Update, update_settings_state);
    }
}

pub fn update_settings_state(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    state: Res<State<SettingState>>,
    mut next_state: ResMut<NextState<SettingState>>,
) {
    if keyboard_input.just_pressed(KeyCode::Escape) {
        match state.as_ref().get() {
            SettingState::Open => next_state.set(SettingState::Closed),
            SettingState::Closed => next_state.set(SettingState::Open),
        }
    }
}
