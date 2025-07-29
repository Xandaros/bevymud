use bevy::prelude::*;

use crate::{
    auth::CharacterLoginEvent,
    misc::{Description, Id},
    telnet::{Connection, EventWriterTelnetEx, SendMessageAction},
};

pub struct RoomPlugin;

impl Plugin for RoomPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Room>()
            .add_observer(on_login)
            .add_observer(on_move_room_action)
            .add_observer(room_enter_description);
    }
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

fn on_move_room_action(trigger: Trigger<MoveRoomAction>, mut commands: Commands) -> Result {
    let target = trigger.target();

    commands.entity(target).insert(InRoom(trigger.new_room));
    commands.trigger_targets(EnterRoomEvent { entity: target }, trigger.new_room);

    Ok(())
}

fn room_enter_description(
    trigger: Trigger<EnterRoomEvent>,
    player_query: Query<&Connection>,
    room_query: Query<(&Name, &Description), With<Room>>,
    mut sender: EventWriter<SendMessageAction>,
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

/// Move target entity into a new room
#[derive(Clone, Debug, Reflect, Event)]
pub struct MoveRoomAction {
    pub new_room: Entity,
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
pub struct InRoom(pub Entity);

#[derive(Component, Debug, Reflect)]
#[relationship_target(relationship = InRoom, linked_spawn)]
pub struct RoomContents(Vec<Entity>);
