use std::time::Duration;

use bevy::{app::ScheduleRunnerPlugin, log::LogPlugin, prelude::*};
use bevy_yarnspinner::prelude::*;
use telnet::{EventWriterTelnetEx, MessageReceived, NewConnection, SendMessage};

mod auth;
mod char;
mod char_creation;
mod database;
mod menu;
mod telnet;
mod util;

fn main() {
    App::new()
        .add_plugins((
            MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(Duration::from_secs_f64(
                1.0 / 60.0,
            ))),
            AssetPlugin::default(),
        ))
        .add_plugins(YarnSpinnerPlugin::new())
        .add_plugins(LogPlugin {
            filter: "info,bevymud=debug".to_string(),
            level: bevy::log::Level::DEBUG,
            custom_layer: |_| None,
        })
        .add_plugins((
            telnet::TelnetPlugin,
            auth::AuthPlugin,
            char_creation::CharCreationPlugin,
            database::DatabasePlugin::new("mysql://test:test@localhost/testing".to_string()),
            menu::MenuPlugin,
        ))
        .add_systems(Update, greet_new)
        .add_systems(Update, echo_control)
        .run();
}

fn greet_new(mut new_conn: EventReader<NewConnection>, mut sender: EventWriter<SendMessage>) {
    for conn in new_conn.read() {
        sender.println(conn.entity, "Hello and welcome!");
        sender.ga(conn.entity);
    }
}

fn echo_control(
    mut message_event: EventReader<MessageReceived>,
    mut sender: EventWriter<SendMessage>,
) {
    for mess in message_event.read() {
        let text = String::from_utf8_lossy(&mess.data);
        if text == "echo off\r\n" {
            sender.echo(mess.connection, false);
        } else if text == "echo on\r\n" {
            sender.echo(mess.connection, true);
        }
    }
}
