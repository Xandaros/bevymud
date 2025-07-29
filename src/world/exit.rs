use bevy::prelude::*;

use crate::{
    player_commands::ExplorationCommandEvent,
    telnet::{EventWriterTelnetEx, SendMessageAction},
};

use super::room::InRoom;

pub struct ExitPlugin;

impl Plugin for ExitPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Exit>()
            .register_type::<OutExit>()
            .register_type::<InExit>()
            .register_type::<InExits>()
            .register_type::<OutExits>()
            .add_observer(exits_command);
    }
}

/// An exit linking two rooms
#[derive(Clone, Debug, Reflect, Component)]
pub struct Exit {
    /// Direction of the exit (e.g. "north", "up", "enter cave", ...)
    pub direction: String,
}

impl Exit {
    pub fn new(direction: impl Into<String>) -> Self {
        Self {
            direction: direction.into(),
        }
    }
}

/// Component placed on exits pointing to which room can reach this exit
#[derive(Clone, Debug, Reflect, Component)]
#[relationship(relationship_target = OutExits)]
pub struct OutExit(pub Entity);

/// Component placed on exits pointing to which room can be reached through this exit
#[derive(Clone, Debug, Reflect, Component)]
#[relationship(relationship_target = InExits)]
pub struct InExit(pub Entity);

/// Component placed on rooms containing a list of exits that can be reached from it
#[derive(Clone, Debug, Deref, Reflect, Component)]
#[relationship_target(relationship = OutExit, linked_spawn)]
pub struct OutExits(Vec<Entity>);

/// Component placed on rooms containing a list of exits leading to this room
#[derive(Clone, Debug, Deref, Reflect, Component)]
#[relationship_target(relationship = InExit, linked_spawn)]
pub struct InExits(Vec<Entity>);

/// List all exits of current room
fn exits_command(
    trigger: Trigger<ExplorationCommandEvent>,
    mut sender: EventWriter<SendMessageAction>,
    room_query: Query<&InRoom>,
    out_exit_query: Query<&OutExits>,
    exit_query: Query<&Exit>,
) -> Result {
    if trigger.command == "exits" {
        let conn = trigger.target();

        let room = room_query.get(conn)?.0;

        let Ok(exits) = out_exit_query.get(room) else {
            sender.println(conn, "No visible exits.");
            return Ok(());
        };

        for exit_ent in &exits.0 {
            if let Ok(exit) = exit_query.get(*exit_ent) {
                sender.println(conn, &exit.direction);
            }
        }
    }

    Ok(())
}
