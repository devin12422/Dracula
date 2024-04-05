mod states;

use bevy::prelude::*;
use bevy_egui::{egui,EguiContexts, EguiPlugin, EguiSettings};
use wasm_bindgen::prelude::*;
use states::*;
use std::time::Duration;
use bevy::time::Stopwatch;
use bevy_egui::egui::ColorImage;
use iyes_progress::prelude::*;

#[derive(Default, Clone)]
pub struct BevyEguiImage{
    name:&'static str,
    handle: Handle<Image>,
    size:egui::Vec2,
}


static SOLAR_ICON_SIZE:egui::Vec2 = egui::Vec2::new(64.0, 64.0);
const SOLAR_RADIUS:f32 = 256.0;
#[derive(Resource)]
struct DayTimer {
    stopwatch: Stopwatch,
    solar_pos:egui::Pos2,
    lunar_pos:egui::Pos2,
}
#[derive(Resource)]
struct SolarSprites {
    solar: BevyEguiImage,
    lunar: BevyEguiImage,
}


fn setup_day_timer(
    mut commands: Commands,
) {
    commands.insert_resource(DayTimer {
        stopwatch: Stopwatch::new(),
        solar_pos: Default::default(),
        lunar_pos: Default::default(),
    })
}
fn setup_game(mut commands:Commands){
    setup_day_timer(commands);

}
fn tick_day_timer(
    time: Res<Time>,
    mut day_timer: ResMut<DayTimer>
) {
    day_timer.stopwatch.tick(time.delta());
    day_timer.solar_pos.x = day_timer.stopwatch.elapsed_secs().cos() * SOLAR_RADIUS;
    day_timer.solar_pos.y = day_timer.stopwatch.elapsed_secs().sin() * SOLAR_RADIUS;
    day_timer.lunar_pos.x = day_timer.solar_pos.x * -1.0;
    day_timer.lunar_pos.y = day_timer.solar_pos.y * -1.0 + SOLAR_RADIUS;
    day_timer.solar_pos.y += SOLAR_RADIUS;
}
#[wasm_bindgen(start)]
pub fn start() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(EguiPlugin).add_plugins(
        ProgressPlugin::new(MyAppState::LoadingScreen)
            .continue_to(MyAppState::InGame)
            .track_assets(),
    )
        .add_systems(Update,(main_menu_gui_system).run_if(in_state(MyAppState::MainMenu)))
        .add_systems(Update,(game_ui_system,tick_day_timer).run_if(in_state(MyAppState::InGame)))
        .add_systems(OnEnter(MyAppState::LoadingScreen),load_game_assets)
        .add_systems(
        Update,
        (
            load_update_system.after(TrackedProgressSet)
        )
            .run_if(in_state(MyAppState::LoadingScreen)),
    )
        .insert_state(MyAppState::MainMenu)
        .insert_state(MyGameState::Home)

        .run();

}
fn game_ui_system(mut contexts: EguiContexts,
                  mut state:ResMut<NextState<MyGameState>>,
                  day_timer:Res<DayTimer>,
                  mut sprites: ResMut<SolarSprites>,
                  image_assets:Res<Assets<Image>>
) {

    egui::Window::new("fdsf").show(contexts.ctx_mut(), |ui| {
        ui.label("dsf");
    });
    egui::TopBottomPanel::top("nav_panel").show(contexts.ctx_mut(), |ui|{
        if ui.button("Go to Work").clicked(){

        }
        egui::Image::from_bytes("moon.png",include_bytes!("../assets/moon.png")).paint_at(ui,egui::Rect::from_center_size(day_timer.solar_pos,SOLAR_ICON_SIZE));
    });

}


fn load_update_system(mut commands:Commands,mut contexts: EguiContexts,    counter: Res<ProgressCounter>,
) {
    let progress = counter.progress();
    egui::Window::new("Loading").show(contexts.ctx_mut(), |ui| {
        ui.label("Loading...{progress.done},{progress.total}");
    });
}
fn load_game_assets(mut commands: Commands,asset_server:Res<AssetServer>,mut loading:ResMut<AssetsLoading>){
    let solar_handle:Handle<Image> = asset_server.load("sun.png");
    let lunar_handle:Handle<Image> = asset_server.load("moon.png");
    loading.add(&solar_handle);
    loading.add(&lunar_handle);
    commands.insert_resource(SolarSprites {
        solar: BevyEguiImage{name:"sun",handle:solar_handle,size:SOLAR_ICON_SIZE},
        lunar: BevyEguiImage{ name:"moon",handle:lunar_handle,size:SOLAR_ICON_SIZE },
    });
    setup_game(commands);

}
fn main_menu_gui_system(mut contexts: EguiContexts,mut state:ResMut<NextState<MyAppState>>) {
    egui::CentralPanel::default().show(contexts.ctx_mut(), |ui|{
        if ui.button("Start").clicked(){
            state.set(MyAppState::LoadingScreen)
        }
        if ui.button("Exit").clicked(){
            state.set(MyAppState::LoadingScreen)
        }
    });
}