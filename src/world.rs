use bevy::prelude::*;

use crate::misc::{Description, Id};

pub mod exit;
pub mod room;

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(room::RoomPlugin);

        app.add_systems(Startup, insert_test_rooms);
    }
}

fn insert_test_rooms(mut commands: Commands) {
    commands.spawn((
        room::Room,
        Name::new("Test"),
        Description::new("A simple room. Nothing to see here."),
        Id(1),
    ));
}
