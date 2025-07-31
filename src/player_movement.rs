use bevy::prelude::*;

use crate::{
    player_commands::ExplorationCommandEvent,
    world::{
        exit::{Exit, InExit, OutExits},
        room::{InRoom, MoveRoomAction},
    },
};

pub struct PlayerMovementPlugin;

impl Plugin for PlayerMovementPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(move_command);
    }
}

fn move_command(
    trigger: Trigger<ExplorationCommandEvent>,
    mut commands: Commands,
    room_query: Query<&InRoom>,
    out_exit_query: Query<&OutExits>,
    exit_query: Query<&Exit>,
    in_exit_query: Query<&InExit>,
) -> Result {
    static ALIASES: phf::Map<&'static str, &'static str> = phf::phf_map! {
        "u" => "up",
        "d" => "down",
        "n" => "north",
        "s" => "south",
        "e" => "east",
        "w" => "west",
    };

    let conn = trigger.target();
    let room = room_query.get(conn)?.0;

    let Ok(exits) = out_exit_query.get(room) else {
        return Ok(());
    };

    for exit_ent in exits.iter() {
        if let Ok(exit) = exit_query.get(exit_ent) {
            if exit.direction == trigger.line
                || ALIASES
                    .get(&trigger.line)
                    .is_some_and(|x| &exit.direction == x)
            {
                let target_room_ent = in_exit_query.get(exit_ent)?.0;
                commands.trigger_targets(
                    MoveRoomAction {
                        old_room: Some(room),
                        new_room: target_room_ent,
                        direction: Some(exit.direction.clone()),
                    },
                    conn,
                );
            }
        }
    }

    Ok(())
}
