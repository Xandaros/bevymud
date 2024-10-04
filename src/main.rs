use std::time::Duration;

use bevy::{app::ScheduleRunnerPlugin, prelude::*};
use telnet::{EventWriterTelnetEx, MessageReceived, NewConnection, SendMessage};

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

fn greet_new(mut new_conn: EventReader<NewConnection>, mut sender: EventWriter<SendMessage>) {
    for conn in new_conn.read() {
        sender.println(conn.entity, "Hello and welcome!");
    }
}

fn echo(mut message_event: EventReader<MessageReceived>, mut sender: EventWriter<SendMessage>) {
    for mess in message_event.read() {
        let text = String::from_utf8_lossy(&mess.data);
        sender.print(mess.connection, &text);
    }
}
