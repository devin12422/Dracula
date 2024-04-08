use std::f32::consts::PI;
use std::ops::Range;

use bevy::prelude::*;
use bevy::utils::HashMap;
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
pub const MIN_ROOM_DIM:f32 = 3.0;
impl BuildingChunk{
    pub fn get_lowest_untaken(&mut self)->Vec<&mut BuildingChunk>{
        match self.divided_chunks {
            Some(ref mut data)=>{
                match data{
                    BuildingChunkData::Tagged => {
                        return Vec::new();
                    },
                    BuildingChunkData::Parent(children) => {
                        let mut vec = Vec::new();
                        for child in children{
                            vec.append(&mut child.get_lowest_untaken());
                        }
                        return vec
                    }
                }
            },
            None=>{
                return vec![self];
            }
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
                   BuildingChunkData::Tagged => {
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
                self.divide_evenly(rng.gen_range((parameters.min_rooms_in_split as f64)..( (parameters.max_rooms_in_split as f64)+1.0)) as usize,parameters.is_hallway,parameters.aspect_ratio_probability_factor,parameters.aspect_ratio_probability_offset,!self.horizontal);
            }, Some(ref mut data) => {
                match data{
                    BuildingChunkData::Tagged => {},
                    BuildingChunkData::Parent(ref mut children) => {
                        for chunk in children {
                            chunk.divide(parameters);
                        }
                    }
                }
            }
        }
    }
    pub fn divide_evenly(&mut self,room_count:usize,is_hallway:bool,aspect_factor:f32,aspect_offset:f32,horizontal:bool){
        if room_count == 0{
            return;
        }
        let mut rng = thread_rng();
        let room_height = if(horizontal){(self.rect.height() - if (is_hallway){(HALL_WIDTH  * (room_count as f32 - 1.0))} else {0.0})/ (room_count as f32)}else{self.rect.height() };
        let room_width = if(horizontal){self.rect.width()}else{(self.rect.width() -if (is_hallway){(HALL_WIDTH  * (room_count as f32 - 1.0))} else {0.0}) / (room_count as f32)};
        let aspect = ((room_width/ (room_height + room_width))-0.5).abs();
        if rng.gen_bool((aspect_offset - (aspect*aspect_factor)).clamp(0.0,1.0) as f64) && room_height > MIN_ROOM_DIM && room_width > MIN_ROOM_DIM{
            for room in 0..room_count{
                if(horizontal){
                    let room_rect=  egui::Rect::from_min_size(Pos2{x:self.rect.left(),y:self.rect.top()+ ((room as f32) * ((if (is_hallway){HALL_WIDTH } else {0.0})+room_height))},emath::Vec2{x:room_width,y:room_height});
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
                    let room_rect =  egui::Rect::from_min_size(Pos2{x:self.rect.left()+ ((room as f32 )* ((if (is_hallway){HALL_WIDTH} else {0.0})+room_width)),y:self.rect.top()},
                                                               emath::Vec2{x:room_width,y:room_height});
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

                }
            }

        }
    }
}

#[derive(Debug)]
pub struct RoomSpec{
    pub(crate) area_range: Rangef,
    pub(crate) door_range:Rangef,
    pub(crate) has_exterior_door:bool,
    pub(crate) room:Option<egui::Rect>
}
impl Default for RoomSpec{
    fn default() -> Self {
        Self{
            area_range:Rangef{min:8.0,max:15.0},
            door_range:Rangef{min:1.0,max:2.0},
            has_exterior_door:false,
            room:None
        }
    }
}
#[derive(Debug)]
pub enum BuildingChunkData{
    Tagged,
    Parent(Vec<BuildingChunk>)
}
const BUILDING_ASPECT_VARIATION:f32 = PI/16.0;
const BUILDING_SIZE_VARIATION:f32 = 2.0;
pub struct BuildingIterationParameters{
    pub min_rooms_in_split:usize,
    pub max_rooms_in_split:usize,
    pub is_hallway:bool,
    pub aspect_ratio_probability_factor:f32,
    pub aspect_ratio_probability_offset:f32,
    pub room_requirements:Vec<RoomSpec>,
}
const MINIMUM_BUILDING_SIZE:f32 = 300.0;
pub fn generate_building(room_iters:Vec<(BuildingIterationParameters,usize)>) -> BuildingChunk {
    let mut area = 0.0;
    let mut rng = thread_rng();
    for iter in &room_iters{
        for room in &iter.0.room_requirements{
            area += (room.area_range.max + room.area_range.min)/2.0;
        }
    }

    area = area.max(MINIMUM_BUILDING_SIZE);
    let theta=rng.gen_range(((PI/4.0)-BUILDING_ASPECT_VARIATION)..((PI/4.0)+BUILDING_ASPECT_VARIATION));
    let scale = rng.gen_range(1.0..(1.0 + BUILDING_SIZE_VARIATION)) ;
    let mut building = BuildingChunk { rect:egui::Rect{min:Pos2{x:0.0,y:0.0},max:Pos2{x:area.sqrt() * theta.cos() * scale,y:area.sqrt() * theta.sin() * scale}}, divided_chunks: None, horizontal: false };
    println!("{:?}", building.rect);

    for specs in room_iters{
        for n in 0..specs.1{
            building.divide(&specs.0);
        }
        for mut room_requirement in specs.0.room_requirements{
            if(room_requirement.room.is_none()){
                let mut candidates = Vec::new();
                for room in building.get_lowest_untaken(){
                    if(room_requirement.has_exterior_door &&  room.rect.intersects(egui::Rect{min:Pos2{x:0.0,y:0.0},max:Pos2{x:area.sqrt() * theta.cos() * scale,y:area.sqrt() * theta.sin() * scale}})) | !(&room_requirement).has_exterior_door{
                        if (room_requirement.area_range.contains(room.rect.area())){
                            candidates.push(room);
                        }
                    }
                }
                for candidate in candidates{
                    if rng.gen_bool((&room_requirement.area_range.center() / (&room_requirement.area_range.center() + candidate.rect.area())) as f64){
                        candidate.divided_chunks = Some(BuildingChunkData::Tagged);
                        room_requirement.room = Some(candidate.rect);
                    }
                }
            }
        }
    }

    building
}