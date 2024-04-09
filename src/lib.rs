#[macro_use]
extern crate static_assertions;
pub mod states;
pub mod work;
mod building;

use serde::{Serialize, Deserialize};
use bevy::pbr::CascadeShadowConfigBuilder;
use rand::prelude::*;
use strum::IntoEnumIterator;
use std::f32::consts::PI;
use std::ops::{Add, Div, Mul};
use std::path::Path;
use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts, EguiPlugin, EguiSettings, EguiUserTextures};
use wasm_bindgen::prelude::*;
use states::*;
use work::*;
use bevy_xpbd_3d::prelude::*;
use bevy::time::Stopwatch;
use bevy::utils::{HashMap};
use bevy_egui::egui::{ Pos2, Rangef, TextureId};
use iyes_progress::prelude::*;
use leafwing_input_manager::prelude::*;
use bevy::window::{CursorGrabMode, PrimaryWindow};
use leafwing_input_manager::prelude::InputKind::Mouse;
use bevy_persistent::prelude::*;
use bevy_ecs::prelude::*;
use bevy_reflect::prelude::*;

const TIME_FACTOR:u32 = 8;
const TOP_UI_HEIGHT_FRACTION:f32 = 7.5;
const SPEED: f32 = 10.0;
pub const DOOR_WIDTH:f32 = 1.0;

static ROTATE_SPEED: f32 = -100.0;

static SOLAR_ICON_SIZE:egui::Vec2 = egui::Vec2::new(64.0, 64.0);

static LUNAR_ICON_SIZE:egui::Vec2 = egui::Vec2::new(32.0, 32.0);
#[derive(Component)]
struct BuildingMarker;
#[derive(Debug,Default,Component)]
pub struct VirtualPointer{
    pub start_click_pos:Option<Vec2>
}
#[derive(PhysicsLayer, Clone, Copy, Debug)]
enum GameLayer {
    Player, // Layer 0
    Environment,
    RaycastInteractible
}
#[derive(Debug, Default, Component)]
pub struct PlayerMarker;
#[derive(Debug, Default, Component)]
pub struct DoorMarker{
    door_enum:DoorEnum
}
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
    pub fn load(&mut self,
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
    Click,
    UIToggle,
    Pause
}
#[derive(Resource, Serialize, Deserialize)]
struct Settings {
    raycast_factor: f64,
}
#[wasm_bindgen(start)]
pub fn start() {
    App::new()
        .add_plugins((DefaultPlugins,EguiPlugin,PhysicsPlugins::default(), PhysicsDebugPlugin::default(), ))
        .add_plugins(
        ProgressPlugin::new(MyAppState::LoadingScreen)
            .continue_to(MyAppState::InGame)
            .track_assets(), )
        .add_systems(Update,(main_menu_gui_system.run_if(in_state(MyAppState::MainMenu)),
                             (game_update_top_ui,
                              game_update,
                              game_update_work.run_if(in_state(MyGameState::Outdoors)).after(game_update_top_ui),
                              mesh_snip_select_system).run_if(in_state(MyAppState::InGame)),
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
    input_map.insert(PlayerMovement::Click, Mouse(MouseButton::Left));
    input_map.insert(PlayerMovement::Look,DualAxis::mouse_motion());
    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_xyz(0.0, 0.9,0.0 )
                .looking_at(Vec3{ x: 1.0, y:0.9, z: 0.0 }, Vec3::Y),
            ..default()
        },
        PlayerMarker,
        )).insert(InputManagerBundle::with_map(input_map));
}

fn paused_update(mut q_windows: Query<&mut Window, With<PrimaryWindow>>,
                 mut next_state:ResMut<NextState<MyAppState>>,
                 mut query: Query<(&mut Transform, &ActionState<PlayerMovement>), With<PlayerMarker>>,
                 // state:Res<State<MyAppState>>,
                 mut contexts: EguiContexts,
                 mut settings:ResMut<Persistent<Settings>>,
                 mut is_settings_open:Local<bool>) {
    let (mut player_transform, mut action_state) = query.single_mut();
    if action_state.just_pressed(&PlayerMovement::Pause){
        let mut primary_window = q_windows.single_mut();
        primary_window.cursor.grab_mode = CursorGrabMode::Locked;
        primary_window.cursor.visible = false;
        next_state.set(MyAppState::InGame);
    }

    if*( is_settings_open){
        egui::Window::new("Settings").show(contexts.ctx_mut(), |ui|{
            if ui.add(egui::Slider::new(&mut settings.raycast_factor, 10.0..=100.0)).changed(){
                settings.persist().expect("failed to save new settings");
            }
        });
    }
    egui::Window::new("Paused").show(contexts.ctx_mut(), |ui| {
        if ui.button("Settings").clicked(){
            *is_settings_open = !(*is_settings_open);
        }
    });
}

fn game_update(mut q_windows: Query<&mut Window, With<PrimaryWindow>>,
               mut contexts:EguiContexts,
               mut query: Query<(&mut Transform, &ActionState<PlayerMovement>), With<PlayerMarker>>,
               mut next_state:ResMut<NextState<MyAppState>>,
               // state:Res<State<MyAppState>>,
               mut pointer: Query<&mut VirtualPointer, With<VirtualPointer>>,
               time:Res<Time>,) {
    let ctx = contexts.ctx_mut();
    let (mut player_transform, mut action_state) = query.single_mut();
    let mut primary_window = q_windows.single_mut();
    if action_state.just_pressed(&PlayerMovement::Pause){
        next_state.set(MyAppState::Paused);
        primary_window.cursor.grab_mode = CursorGrabMode::None;
        primary_window.cursor.visible = true;
        pointer.get_single_mut().unwrap().start_click_pos = None;
    }
    if action_state.just_pressed(&PlayerMovement::UIToggle){
        primary_window.cursor.grab_mode = CursorGrabMode::Confined;
        primary_window.cursor.visible = true;
    } else if action_state.just_released(&PlayerMovement::UIToggle){
        primary_window.cursor.grab_mode = CursorGrabMode::Locked;
        primary_window.cursor.visible = false;
        pointer.get_single_mut().unwrap().start_click_pos = None;
    }
    if(primary_window.cursor.grab_mode == CursorGrabMode::Locked){
        if action_state.pressed(&PlayerMovement::Look) {
            let axis_pair = action_state.axis_pair(&PlayerMovement::Look).unwrap();
            player_transform.rotate_y(axis_pair.x() * time.delta_seconds() / ROTATE_SPEED);
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
    let config_dir = dirs::config_dir()
        .map(|native_config_dir| native_config_dir.join("dracula"))
        .unwrap_or(Path::new("local").join("configuration"));
    commands.insert_resource(
        Persistent::<Settings>::builder()
            .name("settings")
            .format(StorageFormat::Toml)
            .path(config_dir.join("settings.toml"))
            .default(Settings { raycast_factor:10.0})
            .build()
            .expect("failed to initialize settings")
    );
}

fn load_room(mut commands: Commands,mut meshes:ResMut<Assets<Mesh>>,mut materials:ResMut<Assets<StandardMaterial>>, query: Query<Entity, With<BuildingMarker>>){
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
    let specs = vec![
        (BuildingIterationParameters{aspect_ratio_probability_factor:0.3,
            aspect_ratio_probability_offset:1.0,
            min_rooms_in_split:2,
            max_rooms_in_split:4,
            is_hallway:true,
            room_requirements: vec![
                RoomSpec{area_range:Rangef::new(3.0,30.0),
                    has_direct_access: false, .. Default::default()  },
                RoomSpec{area_range:Rangef::new(3.0,30.0),
                    has_direct_access: false, .. Default::default()  },
                RoomSpec{area_range:Rangef::new(30.0,100.0),
                    has_direct_access: true , .. Default::default() }],
        }, 2),
        (BuildingIterationParameters{aspect_ratio_probability_factor:0.7,
            aspect_ratio_probability_offset:1.0,
            min_rooms_in_split:2,
            max_rooms_in_split:3,
            is_hallway:false,
            room_requirements: vec![
                RoomSpec{area_range:Rangef::new(3.0, 30.0),
                    has_direct_access: false,..Default::default() },
                RoomSpec{area_range:Rangef::new(3.0,30.0),
                    has_direct_access: false, .. Default::default()  },],
        }, 4)];

    let mut building = generate_building(specs);
    for chunk in building.get_all(){
        let room = chunk.rect;
        if let Some(BuildingChunkData::Parent(children,is_hallway)) = &chunk.divided_chunks {

        }else{
            commands.spawn((PbrBundle {
                mesh: meshes.add(Cuboid::new(room.width(), 1.0, 0.1)),
                material: materials.add(Color::rgb_u8(124, 144, 255)),
                transform: Transform::from_xyz(room.center_bottom().x, 0.5, room.center_bottom().y),
                ..default()
            },BuildingMarker,RigidBody::Static));
            commands.spawn((PbrBundle {
                mesh: meshes.add(Cuboid::new(room.width(), 1.0, 0.1)),
                material: materials.add(Color::rgb_u8(124, 144, 255)),
                transform: Transform::from_xyz(room.center_top().x, 0.5, room.center_top().y),
                ..default()
            },BuildingMarker,RigidBody::Static));
            commands.spawn((    PbrBundle {
                mesh: meshes.add(Cuboid::new(0.1, 1.0, room.height())),
                material: materials.add(Color::rgb_u8(124, 144, 255)),
                transform: Transform::from_xyz(room.right_center().x, 0.5, room.right_center().y),
                ..default()
            },BuildingMarker,RigidBody::Static));
            commands.spawn((PbrBundle {
                mesh: meshes.add(Cuboid::new(0.1, 1.0, room.height())),
                material: materials.add(Color::rgb_u8(124, 144, 255)),
                transform: Transform::from_xyz(room.left_center().x , 0.5,room.left_center().y),
                ..default()
            },BuildingMarker,RigidBody::Static));
            commands.spawn(PointLightBundle {
                point_light: PointLight{
                    color: Color::rgb(1.0, 1.0, 1.0),
                    intensity: 10000.0,
                    range: room.width().max(room.height()),
                    radius: 0.0,
                    ..Default::default()
                },
                transform: Transform::from_xyz(room.center().x, 1.0,room.center().y ),
                ..Default::default()
            });
            for dir in 0..chunk.doors.len(){
                let angle = dir as f32 * PI / 2.0;
                let angle_vec = Vec2::from_angle(angle);
                let corner_pos = room.center().add(bevy_egui::egui::emath::Vec2{x:(angle_vec.x * room.width()/2.0),y:(angle_vec.y * room.height()/2.0)});
                for door_num in 0..chunk.doors[dir].len(){
                    let mut door_transform = if(angle_vec.y != 0.0){
                        Transform::from_xyz(corner_pos.x - (room.width() / 2.0) + ((room.width()/(chunk.doors[dir].len()) as f32) * ((door_num ) as f32 + 0.5)), 0.5,corner_pos.y )
                    }else{
                        Transform::from_xyz(corner_pos.x , 0.5,corner_pos.y - (room.height() / 2.0) + ((room.height()/(chunk.doors[dir].len()) as f32) * ((door_num ) as f32 + 0.5)))
                    };
                    door_transform.rotate_local_y(angle);
                    commands.spawn((PbrBundle {
                        mesh: meshes.add(Cuboid::new(0.11  , 1.1,DOOR_WIDTH)),
                        material: materials.add(Color::rgb_u8(124, 255, 124)),
                        transform:door_transform,
                        ..default() },
                                    BuildingMarker,
                                    RigidBody::Static,
                                    Collider::cuboid(0.11, 1.1,DOOR_WIDTH)));
                }

            }
        }

    }
}
use crate::building::{BuildingChunkData, BuildingIterationParameters, DoorEnum, generate_building, HALL_WIDTH, RoomSpec};
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
    commands.spawn((
        VirtualPointer { start_click_pos: None },
    ));
    loading.add(&solar_handle);
    loading.add(&lunar_handle);
    // emoji_handle. .typed::<T>()
    commands.insert_resource(TopUISprites {
        solar: BevyEguiImageWrapper{ id: None, handle:solar_handle} ,
        lunar: BevyEguiImageWrapper{ id: None, handle:lunar_handle},
        emoji_map: emojis,
        special_emoji_map: special_emojis,
    });
    commands.insert_resource(AmbientLight {
        color: Color::ORANGE_RED,
        brightness: 0.05,
    });
    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            illuminance: light_consts::lux::AMBIENT_DAYLIGHT,
            shadows_enabled: true,
            ..default()
        },
        transform: Transform {
            translation: Vec3::new(0.0, 2.0, 10.0),
            rotation: Quat::from_rotation_x(-PI / 4.),
            ..default()
        },
        cascade_shadow_config: CascadeShadowConfigBuilder {
            first_cascade_far_bound: 4.0,
            maximum_distance: 10.0,
            ..default()
        }
            .into(),
        ..default()
    });
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
                        mut state:ResMut<NextState<MyAppState>>,
){
    egui::CentralPanel::default().show(contexts.ctx_mut(), |ui|{
        if ui.button("Start").clicked(){
            state.set(MyAppState::LoadingScreen)
        }
        if ui.button("Exit").clicked(){
            app_exit_events.send(bevy::app::AppExit);
        }
    });
}

const RAY_RANGE:f32 = 5.0;
 fn mesh_snip_select_system(
    mut q_windows: Query< &mut Window, With<PrimaryWindow>>,
    mut query: Query<( &ActionState<PlayerMovement>,&GlobalTransform,&Camera), With<PlayerMarker>>,
    mut pointer: Query<&mut VirtualPointer, With<VirtualPointer>>,
    mut egui_contexts: EguiContexts,
    mut gizmos: Gizmos,
    spatial_query: SpatialQuery) {
     let ctx = egui_contexts.ctx_mut();
     let (mut action_state,transform, camera)= query.single_mut();
     let primary_window = q_windows.single_mut();
    // Cast ray and print first hit

    if primary_window.cursor.grab_mode == CursorGrabMode::Confined{
        if action_state.just_pressed(&PlayerMovement::Click){
            pointer.get_single_mut().unwrap().start_click_pos = primary_window.cursor_position();
            println!("{:?}",primary_window.cursor_position().unwrap());
        } else if let Some(start_pointer_pos) = pointer.get_single_mut().unwrap().start_click_pos{
            let end_pointer_pos = primary_window.cursor_position().unwrap();
            // Calculate a ray pointing from the camera into the world based on the cursor's position.
            let Some(left_bottom) = camera.viewport_to_world(&transform, Vec2{x:start_pointer_pos.x.min(end_pointer_pos.x),y:start_pointer_pos.y.min(end_pointer_pos.y)}) else {
                return;
            };
            let Some(right_bottom) = camera.viewport_to_world(&transform, Vec2{x:start_pointer_pos.x.max(end_pointer_pos.x),y:start_pointer_pos.y.min(end_pointer_pos.y)}) else {
                return;
            };
            let Some(right_top) = camera.viewport_to_world(&transform, Vec2{x:start_pointer_pos.x.max(end_pointer_pos.x),y:start_pointer_pos.y.max(end_pointer_pos.y)}) else {
                return;
            };
            let Some(left_top) = camera.viewport_to_world(&transform, Vec2{x:start_pointer_pos.x.min(end_pointer_pos.x),y:start_pointer_pos.y.max(end_pointer_pos.y)}) else {
                return;
            };
            gizmos.linestrip(vec![left_bottom.get_point(RAY_RANGE),
                                  right_bottom.get_point(RAY_RANGE),
                                  right_top.get_point(RAY_RANGE),
                                  left_top.get_point(RAY_RANGE),
                                  left_bottom.get_point(RAY_RANGE)],Color::BLUE);
            if action_state.just_released(&PlayerMovement::Click) {
                println!("released");
                let intersections = spatial_query.shape_intersections(
                    &Collider::trimesh(vec![left_top.origin,
                                            right_top.origin,
                                            left_bottom.origin,
                                            right_bottom.origin,
                                            left_top.get_point(RAY_RANGE),
                                            right_top.get_point(RAY_RANGE),
                                            left_bottom.get_point(RAY_RANGE),
                                            right_bottom.get_point(RAY_RANGE)],
                                       vec![[0, 1, 2], // Side 0
                                            [2, 1, 3],
                                            [4, 0, 6], // Side 1
                                            [6, 0, 2],
                                            [7, 5, 6], // Side 2
                                            [6, 5, 4],
                                            [3, 1, 7], // Side 3
                                            [7, 1, 5],
                                            [4, 5, 0], // Side 4
                                            [0, 5, 1],
                                            [3, 7, 2], // Side 5
                                            [2, 7, 6]]),          // Shape
                    Vec3::ZERO,
                    Quat::default(),
                    SpatialQueryFilter::default(),
                );
                for entity in intersections.iter() {
                    println!("Entity: {:?}", entity);
                }

                pointer.get_single_mut().unwrap().start_click_pos = None;
            }
        }
    }
}