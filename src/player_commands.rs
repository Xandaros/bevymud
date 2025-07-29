use bevy::prelude::*;

use crate::{auth::CharacterLoginEvent, telnet::MessageReceived};

pub struct PlayerCommandsPlugin;

impl Plugin for PlayerCommandsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(FixedUpdate, on_message_received)
            .add_observer(test)
            .add_observer(on_login);
    }
}

fn test(trigger: Trigger<ExplorationCommandEvent>) {
    println!("{:?}", *trigger);
}

/// A player that is in the world, doing normal movement, emotes, etc.
/// (i.e. not in an editor or otherwise in a different "menu")
#[derive(Copy, Clone, Debug, Reflect, Component)]
pub struct Exploring;

#[derive(Clone, Debug, Reflect, Event)]
pub struct ExplorationCommandEvent {
    pub command: String,
    pub args: Vec<String>,
    pub line: String,
}

fn on_login(trigger: Trigger<CharacterLoginEvent>, mut commands: Commands) {
    commands.entity(trigger.target()).insert(Exploring);
}

fn on_message_received(
    mut events: EventReader<MessageReceived>,
    query: Query<(), With<Exploring>>,
    mut commands: Commands,
) {
    for event in events.read() {
        if query.contains(event.connection) {
            let line = event.to_text();
            let mut split = line.split(" ");
            let Some(command) = split.next() else {
                continue;
            };

            commands.trigger_targets(
                ExplorationCommandEvent {
                    command: command.to_string(),
                    args: split.map(ToString::to_string).collect(),
                    line,
                },
                event.connection,
            );
        }
    }
}
