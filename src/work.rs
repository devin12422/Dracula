use std::f32::consts::PI;
use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts, EguiPlugin, EguiSettings, EguiUserTextures};
use wasm_bindgen::prelude::*;
use bevy_egui::egui::{Color32, ColorImage, Pos2, Rangef, Rounding, Stroke, TextureId};
use iyes_progress::prelude::*;
use rand::Rng;
use rnglib::{RNG, Language};
use rand::distributions::uniform::SampleRange;
use serde::{Deserialize, Serialize};
use quadtree_rs::{area::AreaBuilder, point::Point, Quadtree};
const ITERS:usize = 20;
const TAN_VARIATION:f32 = PI/16.0;
const SIZE_VARIATION:f32 = 0.25;
#[derive(Resource)]
struct Client{
    first_name:String,
    last_name:String,
}
impl Client{
    fn generate_client() -> Self{
        let rng = RNG::try_from(&Language::Elven).unwrap();
        Self{
            first_name: rng.generate_name(),
            last_name:rng.generate_name()}
    }
}
#[derive(Resource)]
pub struct HouseLayout{
    pub(crate) rooms:Vec<egui::Rect>,
    pub(crate) quad:Quadtree<u64,String>,
}
pub const DEPTH:usize = (1000_i32).ilog2() as usize;
pub const DIM:usize = 2_usize.pow(DEPTH as u32);
pub fn game_update_work(mut contexts: EguiContexts,
                        mut egui_rooms:ResMut<HouseLayout>, ) {
    egui::CentralPanel::default().show(contexts.ctx_mut(), |ui|{
        if ui.button("dsf").clicked(){
            let mut rng = rand::thread_rng();
            let root_dim = ((DIM as f32).sqrt());
            let theta = rng.gen_range(PI/4.0 - TAN_VARIATION..PI/4.0 + TAN_VARIATION);
            let max = Pos2{ x:theta.cos() * root_dim * rng.gen_range((1.0 - SIZE_VARIATION).. (1.0+ SIZE_VARIATION)),
                y:theta.sin() * root_dim * rng.gen_range((1.0-SIZE_VARIATION)..(1.0+SIZE_VARIATION))};
            let min = Pos2{x:rng.gen_range(0.0..(DIM as f32- max.x) ),y:rng.gen_range(0.0..(DIM as f32-max.y))};
            let region_b = AreaBuilder::default()
                .anchor(Point {x: min.x as u64, y: min.y as u64})
                .dimensions((max.x as u64, max.y as u64))
                .build().unwrap();
            if (egui_rooms.quad.query(region_b).next().is_none()){
                egui_rooms.quad.insert(region_b,"bingo".to_string());
                egui_rooms.rooms.push(egui::Rect::from_min_max(min,max));
            }
        }
        if ui.button("bruh").clicked(){
            for reg in egui_rooms.quad.regions(){
                println!("bottom:{0},top{1},left{2},rught{3}",reg.bottom_edge(),reg.top_edge(),reg.left_edge(),reg.right_edge());
            }
        }
    });
    for room in egui_rooms.rooms.iter(){
        egui::Window::new("bruh").fixed_rect(*room).id(egui::Id::new(format!("x{0},y{1},x{2},y{3}",room.min.x,room.min.y,room.max.x,room.max.y))).show(contexts.ctx_mut(), |ui|{});
        // ui.painter().rect_filled(*room, Rounding::default(), Color32::from_white_alpha(255));
    }
}
// fn generate_building(dim:usize){
//     let root_dim = ((dim as f32).powf(0.5));
//     let mut qt = Quadtree::<u64, String>::new(dim.ilog2() as usize);
//     let mut rng = rand::thread_rng();
//     let mut rooms:Vec<Rect> = Vec::new();


// }