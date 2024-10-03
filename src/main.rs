use std::time::Duration;

use bevy::{app::ScheduleRunnerPlugin, prelude::*};
use telnet::{Connection, MessageReceived, NewConnection};

mod telnet;

fn main() {
    App::new()
        .add_plugins(
            MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(Duration::from_secs_f64(
                1.0 / 60.0,
            ))),
        )
        .add_plugins(telnet::TelnetPlugin)
        .add_systems(Update, greet_new)
        .add_systems(Update, echo)
        .run();
}

fn greet_new(mut new_conn: EventReader<NewConnection>, mut query: Query<&mut Connection>) {
    for conn in new_conn.read() {
        let entity = conn.entity;
        if let Ok(mut connection) = query.get_mut(entity) {
            let events = connection.parser.send_text("Hello and welcome!");
            let _ = connection.telnet_event_sender.try_send(events);
        }
    }
}

fn echo(mut message_event: EventReader<MessageReceived>, mut query: Query<&mut Connection>) {
    for mess in message_event.read() {
        if let Ok(mut connection) = query.get_mut(mess.connection) {
            let text = String::from_utf8_lossy(&mess.data);
            let telnet_event = connection.parser.send_text(&text);
            let _ = connection.telnet_event_sender.try_send(telnet_event);
        }
    }
}
