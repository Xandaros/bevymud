use bevy::prelude::*;
use libmudtelnet::events::TelnetEvents;

use crate::{
    auth::CharacterLoginEvent,
    misc::{Description, Id},
    telnet::{Connection, EventWriterTelnetEx, SendMessage},
};

pub struct RoomPlugin;

impl Plugin for RoomPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Room>()
            .add_observer(on_login)
            .add_observer(room_enter_description)
            .add_systems(Startup, insert_test_rooms);
    }
}

fn insert_test_rooms(mut commands: Commands) {
    commands.spawn((
        Room,
        Name::new("Test"),
        Description::new("A simple room. Nothing to see here."),
        Id(1),
        RoomContents(Vec::new()),
    ));
}

fn on_login(
    trigger: Trigger<CharacterLoginEvent>,
    mut commands: Commands,
    query: Query<(Entity, &Id), With<Room>>,
) {
    let mut room = None;
    for (room_ent, room_id) in query.iter() {
        if room_id.0 == trigger.room {
            room = Some(room_ent);
            break;
        }
    }
    let Some(room) = room else {
        // TODO
        return;
    };

    commands.entity(trigger.target()).insert(InRoom(room));
    commands.trigger_targets(
        EnterRoomEvent {
            entity: trigger.target(),
        },
        room,
    );
}

fn room_enter_description(
    trigger: Trigger<EnterRoomEvent>,
    player_query: Query<&Connection>,
    room_query: Query<(&Name, &Description), With<Room>>,
    mut sender: EventWriter<SendMessage>,
) {
    let room = trigger.target();

    let Ok(conn) = player_query.get(trigger.entity) else {
        // Not a player
        return;
    };

    let Ok((name, description)) = room_query.get(room) else {
        // Room not loaded
        return;
    };

    sender.println(trigger.entity, name.as_str());
    sender.println(trigger.entity, &description.0);
}

// TODO
/// Event that fires during the transition from one room to another
pub struct MoveRoomEvent {
    pub old_room: u64,
    pub new_room: u64,
}

/// Event that fires after a something enters a room
/// Event target is the room
#[derive(Clone, Copy, Reflect, Debug, Event)]
pub struct EnterRoomEvent {
    /// The entity entering the room
    pub entity: Entity,
}

#[derive(Copy, Clone, Debug, Reflect, Component)]
pub struct Room;

#[derive(Component, Debug, Reflect)]
#[relationship(relationship_target = RoomContents)]
pub struct InRoom(Entity);

#[derive(Component, Debug, Reflect)]
#[relationship_target(relationship = InRoom, linked_spawn)]
pub struct RoomContents(Vec<Entity>);
