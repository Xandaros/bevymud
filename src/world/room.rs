use bevy::prelude::*;

use crate::{
    auth::CharacterLoginEvent,
    misc::{Description, Id},
    player_commands::ExplorationCommandEvent,
    telnet::{Connection, EventWriterTelnetEx, SendMessageAction},
};

pub struct RoomPlugin;

impl Plugin for RoomPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Room>()
            .add_observer(on_login)
            .add_observer(on_move_room_action)
            .add_observer(on_show_room_description_action)
            .add_observer(on_look_command)
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

    let mut entity = commands.entity(trigger.target());
    entity.insert(InRoom(room));
    entity.insert(Name::new(trigger.name.clone()));
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

fn room_enter_description(trigger: Trigger<EnterRoomEvent>, mut commands: Commands) {
    commands.trigger_targets(
        ShowRoomDescriptionAction {
            room: trigger.target(),
        },
        trigger.entity,
    );
}

fn on_show_room_description_action(
    trigger: Trigger<ShowRoomDescriptionAction>,
    room_query: Query<(&Name, &Description, Option<&RoomContents>), With<Room>>,
    items_query: Query<&Name>,
    mut sender: EventWriter<SendMessageAction>,
) {
    let conn = trigger.target();

    let Ok((name, description, contents)) = room_query.get(trigger.room) else {
        // Room not loaded
        return;
    };

    sender.println(conn, "");
    sender.print(conn, "\x1b[32m");
    sender.print(conn, name.as_str());
    sender.println(conn, "\x1b[0m");
    sender.println(conn, &description.0);

    if let Some(contents) = contents
        && !contents.0.is_empty()
    {
        sender.println(conn, "");
        sender.println(conn, "You see here:");

        for item in contents.iter() {
            let Ok(name) = items_query.get(item) else {
                continue;
            };
            sender.println(conn, name.as_str());
        }
    }
}

fn on_look_command(
    trigger: Trigger<ExplorationCommandEvent>,
    mut commands: Commands,
    room_query: Query<&InRoom>,
) -> Result {
    if trigger.command == "look" || trigger.command == "l" {
        commands.trigger_targets(
            ShowRoomDescriptionAction {
                room: room_query.get(trigger.target())?.0,
            },
            trigger.target(),
        );
    }
    Ok(())
}

/// Show room description to target player
#[derive(Clone, Debug, Reflect, Event)]
pub struct ShowRoomDescriptionAction {
    pub room: Entity,
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
