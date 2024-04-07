use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use rnglib::{Language, RNG};

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
    egui::CentralPanel::default().show(contexts.ctx_mut(), |ui|{

    });

}
pub fn game_enter_building(mut commands: Commands){

}
