use bevy::prelude::*;
use bevy::ecs::schedule;

#[derive(States,Debug, Clone, PartialEq, Eq, Hash)]
pub enum MyAppState {
    LoadingScreen,
    MainMenu,
    InGame,
}
#[derive(States,Debug, Clone, PartialEq, Eq, Hash)]
pub enum MyGameState {
    Work,
    Home,
    Sleeping,
}