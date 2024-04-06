pub mod states;
pub mod work;

use std::f32::consts::PI;
use std::ops::{Add, Deref};
use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts, EguiPlugin, EguiSettings, EguiUserTextures};
use wasm_bindgen::prelude::*;
use states::*;
use work::*;
use std::ops::Sub;
use std::time::Duration;
use bevy::asset::LoadedFolder;
use bevy::time::Stopwatch;
use bevy_egui::egui::{ColorImage, Pos2, Rangef, TextureId};
use iyes_progress::prelude::*;


static SOLAR_ICON_SIZE:egui::Vec2 = egui::Vec2::new(64.0, 64.0);
static LUNAR_ICON_SIZE:egui::Vec2 = egui::Vec2::new(32.0, 32.0);

struct DayTimer {
    stopwatch: Stopwatch,
    timefactor:u32,
    sleepwatch:Stopwatch,
    solar_pos:egui::Pos2,
    lunar_pos:egui::Pos2,
}
const TIME_FACTOR:u32 = 8;
impl Default for DayTimer {
    fn default() -> Self {
        Self{
            stopwatch:Stopwatch::default(),
            timefactor:TIME_FACTOR,
            sleepwatch:Stopwatch::default(),
            solar_pos:egui::Pos2::default(),
            lunar_pos:egui::Pos2::default()
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
struct SolarSprites {
    solar: BevyEguiImageWrapper,
    lunar: BevyEguiImageWrapper
}
enum Emoji{
    Beaming,
    Concern,
    Confused,
    Crying,
    Frown,
    FrownTear,
    Grin,
    GrinEyes,
    GrinBigEyes,
    Neutral,
    OpenFrown,
    Pensive,
    Relieved,
    SlightFrown,
    SlightSmile,
    Smile,
    SmileEyes,
    SmileTear,
    Stressed,
    SuperWorried,
    Unamused,
    Worried
}
enum SpecialEmoji{
    Yawn,
    Sleep,
    Confounded,
    Fear,
    Sleepy,
    Shock,
    Shaking,
    Partying,
    Monocle,
    Money,
    Melting,
    Heart,
    Eyebrow,
    Eating,
    Dead,
    Star,

}
#[derive(Resource)]
struct PersonState {
    happiness: i32,
    emoji:Emoji,
    special_emoji:Option<SpecialEmoji>,
    money:i32
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

#[wasm_bindgen(start)]
pub fn start() {
    App::new()
        .add_plugins((DefaultPlugins,EguiPlugin)).add_plugins(
        ProgressPlugin::new(MyAppState::LoadingScreen)
            .continue_to(MyAppState::InGame)
            .track_assets(), )
        .add_systems(Update,(main_menu_gui_system.run_if(in_state(MyAppState::MainMenu)),
                             game_update_top_ui.run_if(in_state(MyAppState::InGame)),
                             game_update_work.run_if(in_state(MyGameState::Work)),
                             loading_game_update.after(TrackedProgressSet)
                                 .run_if(in_state(MyAppState::LoadingScreen))))
        .add_systems(OnEnter(MyAppState::LoadingScreen),loading_game_assets_enter)
        .add_systems(OnExit(MyAppState::LoadingScreen),loading_game_assets_exit)
        .insert_state(MyAppState::MainMenu)
        .insert_state(MyGameState::Home)
        .run();

}
const TOP_UI_HEIGHT_FRACTION:f32 = 7.5;
fn game_update_top_ui(mut contexts:EguiContexts, time:Res<Time>,
                      mut day_timer: Local<DayTimer>,
                      sprites: Res<SolarSprites>,
                      state:Res<State<MyGameState>>,
                      mut next_state:ResMut<NextState<MyGameState>>,){
    let screen = &contexts.ctx_mut().screen_rect();
    let time_factor = day_timer.timefactor;
    day_timer.stopwatch.tick(time.delta()/  time_factor as u32);
    day_timer.sleepwatch.tick(time.delta()/ time_factor as u32);
    day_timer.solar_pos.x = (day_timer.stopwatch.elapsed_secs()).cos() * screen.width()/8.0;
    day_timer.solar_pos.y = (day_timer.stopwatch.elapsed_secs()).sin() * screen.height()/TOP_UI_HEIGHT_FRACTION;
    day_timer.lunar_pos.x = day_timer.solar_pos.x * -1.0;
    day_timer.lunar_pos.y = day_timer.solar_pos.y * -1.0;
    egui::TopBottomPanel::top("nav_panel").exact_height(screen.height()/TOP_UI_HEIGHT_FRACTION).show(contexts.ctx_mut(), |ui|{
        ui.label(format!("You are on day # {0}",(day_timer.stopwatch.elapsed_secs()/(2.0*PI)) as u16+1));

        match state.get() {
                    MyGameState::Work => {
                        if ui.button("Go Home").clicked(){
                            next_state.set(MyGameState::Home);
                        }
                    },
                    MyGameState::Home => {
                        if ui.button("Go to Work").clicked(){
                            next_state.set(MyGameState::Work);
                        }
                        if ui.button("Go to Sleep").clicked(){
                            next_state.set(MyGameState::Sleeping);
                            day_timer.timefactor /= TIME_FACTOR;
                            day_timer.sleepwatch.reset();
                        }
                    },
                    MyGameState::Sleeping => {
                        day_timer.sleepwatch.tick(time.delta());
                        if((day_timer.sleepwatch.elapsed_secs() /PI )as u16 > 1){
                            day_timer.sleepwatch.reset();
                            day_timer.timefactor *= TIME_FACTOR;
                            next_state.set(MyGameState::Home);
                        }
                    }
        }
        egui::widgets::Image::new(egui::load::SizedTexture::new(
            sprites.lunar.id.unwrap(),
            LUNAR_ICON_SIZE,
        )).paint_at(ui,egui::Rect::from_center_size(day_timer.solar_pos.add(egui::Vec2{x:screen.width()/2.0,y:screen.height()/TOP_UI_HEIGHT_FRACTION}),LUNAR_ICON_SIZE));
        egui::widgets::Image::new(egui::load::SizedTexture::new(
            sprites.solar.id.unwrap(),
            SOLAR_ICON_SIZE,
        )).paint_at(ui,egui::Rect::from_center_size(day_timer.lunar_pos.add(egui::Vec2{x:screen.width()/2.0,y:screen.height()/TOP_UI_HEIGHT_FRACTION}),SOLAR_ICON_SIZE)); });

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

fn loading_game_assets_enter(mut commands: Commands,
                             asset_server:Res<AssetServer>,
                             mut loading:ResMut<AssetsLoading>,
                             mut egui_user_textures: ResMut<EguiUserTextures>,) {
    let solar_handle:Handle<Image> = asset_server.load("sun.png");
    let lunar_handle:Handle<Image> = asset_server.load("moon.png");
    //let emoji_handle: Handle<LoadedFolder> = asset_server.load_folder("emojis");
    loading.add(&solar_handle);
    loading.add(&lunar_handle);
    //loading.add(&emoji_handle);
    // emoji_handle. .typed::<T>()

    commands.insert_resource(SolarSprites {
        solar: BevyEguiImageWrapper{ id: None, handle:solar_handle} ,
        lunar: BevyEguiImageWrapper{ id: None, handle:lunar_handle}
    });
}
fn loading_game_assets_exit(mut sprites: ResMut<SolarSprites>,
                            mut egui_user_textures: ResMut<EguiUserTextures>) {
    sprites.lunar.load(&mut egui_user_textures);
    sprites.solar.load(&mut egui_user_textures);
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