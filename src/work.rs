use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts, EguiPlugin, EguiSettings, EguiUserTextures};
use wasm_bindgen::prelude::*;
use bevy_egui::egui::{ColorImage, Rangef, TextureId};
use iyes_progress::prelude::*;
use rnglib::{RNG, Language};
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
pub fn game_update_work(mut contexts: EguiContexts, ) {

    egui::Window::new("fdsf").show(contexts.ctx_mut(), |ui| {
        ui.label("dsf"); });
    egui::CentralPanel::default().show(contexts.ctx_mut(), |ui|{
    });
}