use bevy::{ecs::system::SystemId, prelude::*};

use crate::telnet::MessageReceived;

pub struct MessageRequestPlugin;

impl Plugin for MessageRequestPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, run_requested);
    }
}

#[derive(Component)]
pub struct MessageRequest(SystemId<MessageReceived>);

fn run_requested(
    mut commands: Commands,
    mut messages: EventReader<MessageReceived>,
    query: Query<(Entity, &MessageRequest)>,
) {
    for message in messages.read() {
        for (ent_id, request) in query.iter() {
            commands.run_system_with_input(request.0, message.clone());
            commands.entity(ent_id).remove::<MessageRequest>();
        }
    }
}
