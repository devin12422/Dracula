use bevy::prelude::*;
use bevy::ecs::schedule;
pub use strum::IntoEnumIterator;
use strum_macros::EnumIter;


#[derive(States,Debug, Clone, PartialEq, Eq, Hash)]
pub enum MyAppState {
    LoadingScreen,
    MainMenu,
    InGame,
    Paused,

}
#[derive(States,Debug, Clone, PartialEq, Eq, Hash)]
pub enum MyGameState {
    Outdoors,
    Indoors,
    Sleeping,
}

#[derive(EnumIter,Default,strum_macros::Display,States,Debug, Clone, PartialEq, Eq, Hash,)]
pub enum Emoji{
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
    #[default]
    SlightSmile,
    Smile,
    SmileEyes,
    SmileTear,
    Stressed,
    SuperWorried,
    Unamused,
    Worried
}
#[derive(EnumIter,strum_macros::Display,States,Debug, Clone, PartialEq, Eq, Hash)]
pub enum SpecialEmoji{
    Yawn,
    Sleeping,
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