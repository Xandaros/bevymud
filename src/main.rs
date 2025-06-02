use std::time::Duration;

use bevy::{app::ScheduleRunnerPlugin, log::LogPlugin, prelude::*};
use char_creation::CharCreationState;
use telnet::{EventWriterTelnetEx, MessageReceived, NewConnection, SendMessage};

mod auth;
mod char;
mod char_creation;
mod database;
mod telnet;
mod util;

fn main() {
    App::new()
        .add_plugins(
            MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(Duration::from_secs_f64(
                1.0 / 60.0,
            ))),
        )
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
        ))
        .add_systems(Update, greet_new)
        .add_systems(Update, echo_control)
        .add_systems(Update, debug)
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

fn debug(
    query: Query<(&telnet::Connection, &auth::Username, &CharCreationState)>,
    time: Res<Time>,
    mut timer: Local<Timer>,
) {
    if timer.mode() == TimerMode::Once {
        timer.set_mode(TimerMode::Repeating);
        timer.set_duration(Duration::from_secs(1));
    }
    timer.tick(time.delta());

    if timer.just_finished() {
        for (conn, username, state) in query.iter() {
            // println!("{:#?}", conn);
            // println!("{:?}", username);
            // println!("{:?}", state);
        }
    }
}
