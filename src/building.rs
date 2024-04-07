use std::f32::consts::PI;

use bevy::prelude::*;
use bevy_egui::egui;
use bevy_egui::egui::{emath, Pos2, Rangef};
use rand::{Rng, thread_rng};

pub const HALL_WIDTH:f32 = 2.0;

#[derive(Debug)]
pub struct BuildingChunk{
    pub rect:egui::Rect,
    pub divided_chunks:Option<BuildingChunkData>,
    pub horizontal:bool
}
pub const MIN_ROOM_DIM:f32 = 5.0;
impl BuildingChunk{
    pub fn get_lowest_untaken<'a>(&'a mut self, vec:&mut Vec<&'a mut Option<BuildingChunkData>>){
        if let None = &self.divided_chunks {
            vec.push(&mut self.divided_chunks);
        }
    }
    pub fn get_lowest_rects(&self)->Vec<egui::Rect>{
        match &self.divided_chunks{
            None=>{
                vec![self.rect]
            },
            Some(T)=>{
                match T{
                    BuildingChunkData::Parent( children)=>{
                        let mut vec = Vec::new();
                        for chunk in children{
                            vec = [vec,chunk.get_lowest_rects()].concat();
                        }
                        vec
                    },
                   BuildingChunkData::Tagged(_spec) => {
                       vec![self.rect]
                   }
                }
            }
        }
    }
    pub fn divide(&mut self,parameters:&BuildingIterationParameters){
        let mut rng = thread_rng();
        match self.divided_chunks{
            None => {
                self.divide_evenly(rng.gen_range((parameters.min_rooms_in_split as f64)..( (parameters.max_rooms_in_split as f64)+1.0)) as usize,parameters.divider_width,parameters.aspect_ratio_probability_factor,parameters.aspect_ratio_probability_offset,!self.horizontal);
            }, Some(ref mut data) => {
                match data{
                    BuildingChunkData::Tagged(_) => {},
                    BuildingChunkData::Parent(ref mut children) => {
                        for chunk in children {
                            chunk.divide(parameters);
                        }
                    }
                }
            }
        }
    }
    pub fn divide_evenly(&mut self,room_count:usize,divider_width:f32,aspect_factor:f32,aspect_offset:f32,horizontal:bool){
        if room_count == 0{
            return;
        }
        let mut rng = thread_rng();
        let room_height = if(horizontal){(self.rect.height() - (divider_width * (room_count as f32 - 1.0)))/ (room_count as f32)}else{self.rect.height() };
        let room_width = if(horizontal){self.rect.width()}else{(self.rect.width() -(divider_width * (room_count as f32- 1.0))) / (room_count as f32)};
        let aspect = ((room_width/ (room_height + room_width))-0.5).abs();
        if rng.gen_bool((aspect_offset - (aspect*aspect_factor)).clamp(0.01,0.99) as f64) && room_height > MIN_ROOM_DIM && room_width > MIN_ROOM_DIM{
            for room in 0..room_count{
                if(horizontal){
                    let room_rect=  egui::Rect::from_min_size(Pos2{x:self.rect.left(),y:self.rect.top()+ ((room as f32) * (divider_width+room_height))},emath::Vec2{x:room_width,y:room_height});
                    match self.divided_chunks{
                        Some(ref mut T)=>{
                            if let BuildingChunkData::Parent(ref mut children) = T {
                                children.push(BuildingChunk{rect:room_rect, divided_chunks: None, horizontal });
                            }
                        },
                        None=>{
                            self.divided_chunks = Some(BuildingChunkData::Parent(vec![BuildingChunk{rect:room_rect, divided_chunks: None, horizontal }]));
                        }
                    };

                }else{
                    let room_rect =  egui::Rect::from_min_size(Pos2{x:self.rect.left()+ ((room as f32 )* (divider_width+room_width)),y:self.rect.top()},
                                                               emath::Vec2{x:room_width,y:room_height});
                    match self.divided_chunks{
                        Some(ref mut T)=>{
                            if let BuildingChunkData::Parent(ref mut children) = T {
                                children.push(BuildingChunk { rect: room_rect, divided_chunks: None, horizontal });
                            };
                        },
                        None=>{
                            self.divided_chunks = Some(BuildingChunkData::Parent(vec![BuildingChunk{rect:room_rect, divided_chunks: None, horizontal }]));
                        }
                    };
                }
            }
        }
    }
}

#[derive(Debug)]
pub struct RoomSpec{
    pub(crate) area_range: Rangef,
    pub(crate) max_doors:usize
}
impl Default for RoomSpec{
    fn default() -> Self {
        todo!()
    }
}
#[derive(Debug)]
pub enum BuildingChunkData{
    Tagged(RoomSpec),
    Parent(Vec<BuildingChunk>)
}
const BUILDING_ASPECT_VARIATION:f32 = PI/16.0;
const BUILDING_SIZE_VARIATION:f32 = 2.0;
pub struct BuildingIterationParameters{
    pub min_rooms_in_split:usize,
    pub max_rooms_in_split:usize,
    pub divider_width:f32,
    pub aspect_ratio_probability_factor:f32,
    pub aspect_ratio_probability_offset:f32,
}
const MINIMUM_BUILDING_SIZE:f32 = 300.0;
pub fn generate_building(room_iters:Vec<(BuildingIterationParameters,usize)>,mut room_req:Vec<RoomSpec>) -> BuildingChunk {
    let mut area = 0.0;
    let mut rng = thread_rng();
    for room in &room_req{
        area += (room.area_range.max + room.area_range.min)/2.0;
    }
    area = area.max(MINIMUM_BUILDING_SIZE);
    let theta=rng.gen_range(((PI/4.0)-BUILDING_ASPECT_VARIATION)..((PI/4.0)+BUILDING_ASPECT_VARIATION));
    let scale = rng.gen_range(1.0..(1.0 + BUILDING_SIZE_VARIATION)) ;
    let rect = egui::Rect{min:Pos2{x:0.0,y:0.0},max:Pos2{x:area.sqrt() * theta.cos() * scale,y:area.sqrt() * theta.sin() * scale}};
    let mut building = BuildingChunk { rect, divided_chunks: None, horizontal: false };
    for specs in room_iters{
        for n in 0..specs.1{
            building.divide(&specs.0);
        }
    }

    building
}