use std::time::Duration;

use bevy::{
    app::ScheduleRunnerPlugin,
    ecs::{archetype::Archetype, world::DeferredWorld},
    log::LogPlugin,
    prelude::*,
};
use bevy_yarnspinner::prelude::*;
use libmudtelnet::events::TelnetEvents;
use player_commands::ExplorationCommandEvent;
use telnet::{EventWriterTelnetEx, MessageReceived, NewConnection, SendMessageAction};

mod auth;
mod char;
mod char_creation;
mod class;
mod database;
mod menu;
mod misc;
mod player_commands;
mod player_movement;
mod race;
mod telnet;
mod util;
mod world;

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
            race::RacePlugin,
            class::ClassPlugin,
            auth::AuthPlugin,
            char_creation::CharCreationPlugin,
            database::DatabasePlugin::new("mysql://test:test@localhost/testing".to_string()),
            menu::MenuPlugin,
            misc::MiscPlugin,
            player_commands::PlayerCommandsPlugin,
            player_movement::PlayerMovementPlugin,
            world::WorldPlugin,
        ))
        .add_systems(Update, greet_new)
        .add_systems(Update, echo_control)
        .add_observer(quit_command)
        .add_observer(debug_command)
        .run();
}

fn greet_new(mut new_conn: EventReader<NewConnection>, mut sender: EventWriter<SendMessageAction>) {
    for conn in new_conn.read() {
        sender.println(conn.entity, "Hello and welcome!");
        sender.ga(conn.entity);
    }
}

fn quit_command(trigger: Trigger<ExplorationCommandEvent>, mut commands: Commands) {
    if trigger.command == "quit" {
        commands.entity(trigger.target()).despawn();
    }
}

fn debug_command(trigger: Trigger<ExplorationCommandEvent>, mut world: DeferredWorld) {
    if trigger.command != "debug" {
        return;
    }

    let conn = trigger.target();

    let mut events = Vec::new();

    for entity in world.iter_entities() {
        events.push(SendMessageAction {
            connection: conn,
            data: TelnetEvents::DataSend(libmudtelnet::Parser::escape_iac(format!(
                "\n\x1b[38;5;1m{}\x1b[0m\n",
                entity.id()
            ))),
        });

        let archetype = entity.components::<&Archetype>();

        for component in archetype.components() {
            let Some(name) = world.components().get_name(component) else {
                continue;
            };

            let mut value = String::new();
            if let Some(info) = world.components().get_info(component) {
                if let Some(type_id) = info.type_id() {
                    if let Ok(val) = world.get_reflect(entity.id(), type_id) {
                        value = format!(": {:#?}", val);
                    }
                }
            }

            events.push(SendMessageAction {
                connection: conn,
                data: TelnetEvents::DataSend(libmudtelnet::Parser::escape_iac(format!(
                    "{}{}\n",
                    &name[name.rfind(":").map(|x| x + 1).unwrap_or(0)..],
                    value
                ))),
            });

            //world.get_reflect(entity.id(), component.into());
        }
    }

    for event in events {
        world.send_event(event);
    }
}

fn echo_control(
    mut message_event: EventReader<MessageReceived>,
    mut sender: EventWriter<SendMessageAction>,
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
