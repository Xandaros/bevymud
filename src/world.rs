use bevy::prelude::*;
use exit::{Exit, InExit, OutExit};

use crate::misc::{Description, Id};

pub mod exit;
pub mod room;

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((room::RoomPlugin, exit::ExitPlugin));

        app.add_systems(Startup, insert_test_rooms);
    }
}

fn insert_test_rooms(mut commands: Commands) {
    let room1 = commands
        .spawn((
            room::Room,
            Name::new("Test"),
            Description::new("A simple room. Nothing to see here."),
            Id(1),
        ))
        .id();
    let room2 = commands
        .spawn((
            room::Room,
            Name::new("Test2"),
            Description::new("Another room."),
            Id(2),
        ))
        .id();
    commands.spawn((Exit::new("north"), OutExit(room1), InExit(room2)));
    commands.spawn((Exit::new("south"), OutExit(room2), InExit(room1)));
}
