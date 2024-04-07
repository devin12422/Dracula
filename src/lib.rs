#[macro_use]
extern crate static_assertions;
pub mod states;
pub mod work;
use rand::prelude::*;
use std::cell::RefCell;

use std::cmp::max;
use strum::IntoEnumIterator;
use std::f32::consts::PI;
use std::ops::{Add, Deref, Div, Mul};
use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts, EguiPlugin, EguiSettings, EguiUserTextures};
use wasm_bindgen::prelude::*;
use states::*;
use work::*;
use std::ops::Sub;
use std::rc::{Rc, Weak};
use bevy::time::Stopwatch;
use bevy::utils::{HashMap, HashSet};
use bevy_egui::egui::{ColorImage, Pos2, Rangef, TextureId};
use iyes_progress::prelude::*;
use leafwing_input_manager::prelude::*;
use bevy::window::{CursorGrabMode, PrimaryWindow};

const TIME_FACTOR:u32 = 8;

const TOP_UI_HEIGHT_FRACTION:f32 = 7.5;

const SPEED: f32 = 10.0;
static ROTATE_SPEED: f32 = -100.0;

static SOLAR_ICON_SIZE:egui::Vec2 = egui::Vec2::new(64.0, 64.0);

static LUNAR_ICON_SIZE:egui::Vec2 = egui::Vec2::new(32.0, 32.0);


#[derive(Debug, Default, Component)]
pub struct PlayerMarker;
struct DayTimer {
    stopwatch: Stopwatch,
    timefactor:u32,
    sleepwatch:Stopwatch,
    solar_pos:egui::Pos2,
    person_state:PersonState,
    happiness: i32,
    money:i32
}

impl Default for DayTimer {
    fn default() -> Self {
        Self{
            stopwatch:Stopwatch::default(),
            timefactor:TIME_FACTOR,
            sleepwatch:Stopwatch::default(),
            solar_pos:egui::Pos2::default(),
            person_state:PersonState::default(),
            happiness: 1,
            money: 100,
        }
    }
}
#[derive(Default)]
struct BevyEguiImageWrapper{
    id:Option<TextureId>,
    handle:Handle<Image>
}
impl BevyEguiImageWrapper{
    fn load(&mut self,
            mut egui_user_textures: &mut ResMut<EguiUserTextures>){
        self.id = Some(egui_user_textures.add_image(self.handle.clone_weak()));
    }
}



#[derive(Resource)]
struct TopUISprites {
    solar: BevyEguiImageWrapper,
    lunar: BevyEguiImageWrapper,
    emoji_map:HashMap<Emoji,BevyEguiImageWrapper>,
    special_emoji_map:HashMap<SpecialEmoji,BevyEguiImageWrapper>,
}
#[derive(Default)]
struct PersonState {
    emoji:Emoji,
    special_emoji:Option<SpecialEmoji>,
}
#[derive(Resource)]
struct EmojiSprites {
    smile: BevyEguiImageWrapper,
    smile_eyes: BevyEguiImageWrapper,
    pensive: BevyEguiImageWrapper,
    worried: BevyEguiImageWrapper,
    shock: BevyEguiImageWrapper,
    yawn: BevyEguiImageWrapper,
    frown: BevyEguiImageWrapper,
    neutral: BevyEguiImageWrapper,
}

#[derive(Actionlike, PartialEq, Eq, Hash, Clone, Copy, Debug, Reflect)]
enum PlayerMovement {
    Look,
    Move,
    UIToggle,
    Pause
}
#[wasm_bindgen(start)]
pub fn start() {
    App::new()
        .add_plugins((DefaultPlugins,EguiPlugin))
        .add_plugins(
        ProgressPlugin::new(MyAppState::LoadingScreen)
            .continue_to(MyAppState::InGame)
            .track_assets(), )
        .add_systems(Update,(main_menu_gui_system.run_if(in_state(MyAppState::MainMenu)),
                             (game_update_top_ui,
                              game_update,
                              game_update_work.run_if(in_state(MyGameState::Outdoors)).after(game_update_top_ui)).run_if(in_state(MyAppState::InGame)),
                             paused_update.run_if(in_state(MyAppState::Paused)),
                             loading_game_update.after(TrackedProgressSet)
                                 .run_if(in_state(MyAppState::LoadingScreen))))
        .insert_state(MyGameState::Outdoors)
        .add_systems(OnEnter(MyGameState::Indoors),load_room)
        .add_systems(OnEnter(MyAppState::LoadingScreen),(loading_game_assets_enter))
        .add_systems(OnExit(MyAppState::LoadingScreen),loading_game_assets_exit)
        .insert_state(MyAppState::MainMenu)
        .add_plugins(InputManagerPlugin::<PlayerMovement>::default())
        .run();

}
fn setup_camera(mut commands: Commands) {
    let mut input_map = InputMap::default();
    input_map.insert(PlayerMovement::Move, VirtualDPad::wasd());
    input_map.insert(PlayerMovement::UIToggle, KeyCode::ShiftLeft);
    input_map.insert(PlayerMovement::Pause, KeyCode::Escape);

    input_map.insert(PlayerMovement::Look,DualAxis::mouse_motion());
    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_xyz(0.0, 0.0, 5.0)
                .looking_at(Vec3{ x: 0.0, y: 0.0, z: 0.0 }, Vec3::Z),
            ..default()
        },
        PlayerMarker,
    ))        .insert(InputManagerBundle::with_map(input_map));
}

fn paused_update(mut q_windows: Query<&mut Window, With<PrimaryWindow>>,
                 mut next_state:ResMut<NextState<MyAppState>>,
                 mut query: Query<(&mut Transform, &ActionState<PlayerMovement>), With<PlayerMarker>>,
                 state:Res<State<MyAppState>>,
                 mut app_exit_events: ResMut<Events<bevy::app::AppExit>>,
                 mut contexts: EguiContexts,) {
    let (mut player_transform, mut action_state) = query.single_mut();
    if action_state.just_pressed(&PlayerMovement::Pause){
        let mut primary_window = q_windows.single_mut();
        primary_window.cursor.grab_mode = CursorGrabMode::Locked;
        primary_window.cursor.visible = false;
        next_state.set(MyAppState::InGame);
    }
    egui::Window::new("Paused").show(contexts.ctx_mut(), |ui| {
        if ui.button("Exit").clicked(){
            app_exit_events.send(bevy::app::AppExit);
        }
    });

}
fn game_update(mut q_windows: Query<&mut Window, With<PrimaryWindow>>,
               mut query: Query<(&mut Transform, &ActionState<PlayerMovement>), With<PlayerMarker>>,
               mut next_state:ResMut<NextState<MyAppState>>,
               state:Res<State<MyAppState>>,
               time:Res<Time>,) {
    let (mut player_transform, mut action_state) = query.single_mut();
    let mut primary_window = q_windows.single_mut();
    if action_state.just_pressed(&PlayerMovement::Pause){
        next_state.set(MyAppState::Paused);
        primary_window.cursor.grab_mode = CursorGrabMode::None;
        primary_window.cursor.visible = true;
    }
    if action_state.just_pressed(&PlayerMovement::UIToggle){
        primary_window.cursor.grab_mode = CursorGrabMode::Confined;
        primary_window.cursor.visible = true;
    } else if action_state.just_released(&PlayerMovement::UIToggle){
        primary_window.cursor.grab_mode = CursorGrabMode::Locked;
        primary_window.cursor.visible = false;
    }
    if(primary_window.cursor.grab_mode == CursorGrabMode::Locked){
        if action_state.pressed(&PlayerMovement::Look) {
            let axis_pair = action_state.axis_pair(&PlayerMovement::Look).unwrap();
            player_transform.rotate_z(axis_pair.x() * time.delta_seconds() / ROTATE_SPEED);
        }
        if action_state.pressed(&PlayerMovement::Move) {
            let axis_pair = action_state.clamped_axis_pair(&PlayerMovement::Move).unwrap();
            let forward = &player_transform.forward();
            let left = &player_transform.left();
            player_transform.translation += forward.mul(axis_pair.y() * time.delta().as_secs_f32() * SPEED);
            player_transform.translation -= left.mul(axis_pair.x() * time.delta().as_secs_f32() * SPEED);
        }
    }else{
        if action_state.pressed(&PlayerMovement::Move) {
            let axis_pair = action_state.clamped_axis_pair(&PlayerMovement::Move).unwrap();
            let forward = &player_transform.forward();
            let left = &player_transform.left();
            player_transform.translation += forward.mul(axis_pair.y() * time.delta().as_secs_f32() * SPEED /2.0);
            player_transform.translation -= left.mul(axis_pair.x() * time.delta().as_secs_f32() * SPEED/2.0);
        }
    }
}
fn game_update_top_ui(mut contexts:EguiContexts,
                      mut next_state:ResMut<NextState<MyGameState>>,
                      mut day_timer: Local<DayTimer>,
                      time:Res<Time>,
                      sprites: Res<TopUISprites>,
                      state:Res<State<MyGameState>>, ){
    let screen = &contexts.ctx_mut().screen_rect();
    let time_factor = day_timer.timefactor;
    day_timer.stopwatch.tick(time.delta()/  time_factor as u32);
    day_timer.sleepwatch.tick(time.delta()/ time_factor as u32);
    day_timer.solar_pos.x = (day_timer.stopwatch.elapsed_secs()).cos() * screen.width()/8.0;
    day_timer.solar_pos.y = (day_timer.stopwatch.elapsed_secs()).sin() * screen.height()/TOP_UI_HEIGHT_FRACTION;
    egui::TopBottomPanel::top("nav_panel").exact_height(screen.height()/TOP_UI_HEIGHT_FRACTION).show(contexts.ctx_mut(), |ui| {
        ui.label(format!("You are on day # {0}", (day_timer.stopwatch.elapsed_secs() / (2.0 * PI)) as u16 + 1));
        match state.get() {
            MyGameState::Outdoors => {
                if ui.button("Go Home").clicked() {
                    next_state.set(MyGameState::Indoors);
                }
            },
            MyGameState::Indoors => {
                if ui.button("Go to Work").clicked() {
                    next_state.set(MyGameState::Outdoors);
                }
                if ui.button("Go to Sleep").clicked() {
                    next_state.set(MyGameState::Sleeping);
                    day_timer.timefactor /= TIME_FACTOR;
                    day_timer.sleepwatch.reset();
                }
            },
            MyGameState::Sleeping => {
                day_timer.sleepwatch.tick(time.delta());
                if ((day_timer.sleepwatch.elapsed_secs() / PI) as u16 > 1) {
                    day_timer.sleepwatch.reset();
                    day_timer.timefactor *= TIME_FACTOR;
                    next_state.set(MyGameState::Indoors);
                }
            }
        }
        if((day_timer.stopwatch.elapsed_secs() /  PI) as u16 %2 == 0){
            egui::widgets::Image::new(egui::load::SizedTexture::new(
                sprites.solar.id.unwrap(),
                SOLAR_ICON_SIZE,
            )).paint_at(ui, egui::Rect::from_center_size(day_timer.solar_pos.mul(-1.0).add(egui::Vec2 { x: screen.width() / 2.0, y: screen.height() / TOP_UI_HEIGHT_FRACTION }), SOLAR_ICON_SIZE));
        }else{
            egui::widgets::Image::new(egui::load::SizedTexture::new(
                sprites.lunar.id.unwrap(),
                SOLAR_ICON_SIZE.div(egui::Vec2{x:2.0,y:2.0}),
            )).paint_at(ui, egui::Rect::from_center_size(day_timer.solar_pos.add(egui::Vec2 { x: screen.width() / 2.0, y: screen.height() / TOP_UI_HEIGHT_FRACTION }), LUNAR_ICON_SIZE));

        }
        if day_timer.person_state.special_emoji.is_some() {
            egui::widgets::Image::new(egui::load::SizedTexture::new(
                sprites.special_emoji_map.get(&day_timer.person_state.special_emoji.clone().unwrap()).unwrap().id.unwrap(),
                SOLAR_ICON_SIZE,
            )).paint_at(ui, egui::Rect::from_center_size(egui::Pos2 { x: screen.width() / 2.0, y: screen.height() / TOP_UI_HEIGHT_FRACTION - (SOLAR_ICON_SIZE.y/2.0)}, SOLAR_ICON_SIZE));
        } else {
            egui::widgets::Image::new(egui::load::SizedTexture::new(
                sprites.emoji_map.get(&day_timer.person_state.emoji).unwrap().id.unwrap(),
                SOLAR_ICON_SIZE,
            )).paint_at(ui, egui::Rect::from_center_size(egui::Pos2 { x: screen.width() / 2.0, y: screen.height() / TOP_UI_HEIGHT_FRACTION - (SOLAR_ICON_SIZE.y/2.0)}, SOLAR_ICON_SIZE));
        }
    });
}

fn loading_game_update(mut commands:Commands,
    mut contexts: EguiContexts,
    counter: Res<ProgressCounter>,
    loading:Res<AssetsLoading>, ){
    let progress = counter.progress();
    egui::Window::new("Loading").show(contexts.ctx_mut(), |ui| {
        ui.label(format!("Loading...{0}/{1}", progress.done,progress.total));
    });
}
fn load_room(mut commands: Commands,mut meshes:ResMut<Assets<Mesh>>,mut materials:ResMut<Assets<StandardMaterial>>,mut egui_rooms:ResMut<HouseLayout>){
    for room_un in &egui_rooms.rooms{
        let room = room_un.div(2.0);
        commands.spawn(PbrBundle {
            mesh: meshes.add(Cuboid::new(room.width(), 1.0, 1.0)),
            material: materials.add(Color::rgb_u8(124, 144, 255)),
            transform: Transform::from_xyz(room.center_bottom().x, room.bottom(), 0.5),
            ..default()
        });
        commands.spawn(PbrBundle {
            mesh: meshes.add(Cuboid::new(room.width(), 1.0, 1.0)),
            material: materials.add(Color::rgb_u8(124, 144, 255)),
            transform: Transform::from_xyz(room.center_top().x, room.top(), 0.5),
            ..default()
        });
        commands.spawn(PbrBundle {
            mesh: meshes.add(Cuboid::new(1.0, room.height(), 1.0)),
            material: materials.add(Color::rgb_u8(124, 144, 255)),
            transform: Transform::from_xyz(room.left(), room.left_center().y, 0.5),
            ..default()
        });
        commands.spawn(PbrBundle {
            mesh: meshes.add(Cuboid::new(1.0, room.height(), 1.0)),
            material: materials.add(Color::rgb_u8(124, 144, 255)),
            transform: Transform::from_xyz(room.right(), room.right_center().y, 0.5),
            ..default()
        });
        commands.spawn(PointLightBundle {
            point_light: PointLight {
                shadows_enabled: true,
                ..default()
            },
            transform: Transform::from_xyz(room.center().x, room.center().y, 4.0),
            ..default()
        });
    }

    // light

}
use quadtree_rs::{area::AreaBuilder, point::Point, Quadtree};


fn loading_game_assets_enter(mut q_windows: Query<&mut Window, With<PrimaryWindow>>,
                             mut commands: Commands,
                             asset_server:Res<AssetServer>,
                             mut loading:ResMut<AssetsLoading>,
                             mut egui_user_textures: ResMut<EguiUserTextures>,
                             mut meshes: ResMut<Assets<Mesh>>,
                             mut materials: ResMut<Assets<StandardMaterial>>,) {

    let mut primary_window = q_windows.single_mut();
    primary_window.cursor.grab_mode = CursorGrabMode::Locked;
    primary_window.cursor.visible = false;

    let solar_handle:Handle<Image> = asset_server.load("sun.png");
    let lunar_handle:Handle<Image> = asset_server.load("moon.png");
    let mut emojis:HashMap<Emoji,BevyEguiImageWrapper>= HashMap::new();
    let mut special_emojis:HashMap<SpecialEmoji,BevyEguiImageWrapper> = HashMap::new();
    for emoji in Emoji::iter(){
        let handle:Handle<Image> = asset_server.load(format!("emojis/{0}.png",&emoji.to_string()));
        loading.add(&handle);
        emojis.insert(emoji,BevyEguiImageWrapper{id:None,handle:handle},);
    }
    for emoji in SpecialEmoji::iter(){
        let handle:Handle<Image> = asset_server.load(format!("emojis/special/{0}.png",&emoji.to_string()));
        loading.add(&handle);
        special_emojis.insert(emoji,BevyEguiImageWrapper{id:None,handle:handle},);

    }
    loading.add(&solar_handle);
    loading.add(&lunar_handle);
    // emoji_handle. .typed::<T>()
    commands.insert_resource(TopUISprites {
        solar: BevyEguiImageWrapper{ id: None, handle:solar_handle} ,
        lunar: BevyEguiImageWrapper{ id: None, handle:lunar_handle},
        emoji_map: emojis,
        special_emoji_map: special_emojis,
    });
    commands.insert_resource(HouseLayout{rooms:Vec::new(),quad:Quadtree::<u64,String>::new(DEPTH)});
    // circular base

    // light

    setup_camera(commands);
}
fn loading_game_assets_exit(mut sprites: ResMut<TopUISprites>,
                            mut egui_user_textures: ResMut<EguiUserTextures>) {
    sprites.lunar.load(&mut egui_user_textures);
    sprites.solar.load(&mut egui_user_textures);
    for (emoji,mut image) in sprites.emoji_map.iter_mut(){
        image.load(&mut egui_user_textures);
    }
    for (emoji,mut image) in sprites.special_emoji_map.iter_mut(){
        image.load(&mut egui_user_textures);
    }
}
fn main_menu_gui_system(mut app_exit_events: ResMut<Events<bevy::app::AppExit>>,
                        mut contexts: EguiContexts,
                        mut state:ResMut<NextState<MyAppState>>) {
    egui::CentralPanel::default().show(contexts.ctx_mut(), |ui|{
        if ui.button("Start").clicked(){
            state.set(MyAppState::LoadingScreen)
        }
        if ui.button("Exit").clicked(){
            app_exit_events.send(bevy::app::AppExit);
        }
    });
}