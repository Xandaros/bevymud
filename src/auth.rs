use std::fmt::Display;

use bevy::prelude::*;

use crate::{
    char_creation::CharCreationState,
    telnet::{EventWriterTelnetEx, MessageReceived, NewConnection, SendMessage},
};

pub struct AuthPlugin;

impl Plugin for AuthPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Username>();
        app.register_type::<LoggedIn>();
        app.add_systems(Update, request_username);
        app.add_systems(Update, request_password);
        app.add_systems(Update, login);
    }
}

#[derive(Component, Clone, Debug, Reflect)]
pub struct Username(pub String);

impl Display for Username {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl Username {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Component, Reflect)]
pub struct LoggedIn;

fn request_username(
    mut new_conn: EventReader<NewConnection>,
    mut sender: EventWriter<SendMessage>,
) {
    for conn in new_conn.read() {
        sender.println(conn.entity, "Welcome to test MUD 1234!");
        sender.println(
            conn.entity,
            "Please enter you username or type NEW to create a new character",
        );
        sender.print(conn.entity, "Username: ");
        sender.ga(conn.entity);
    }
}

fn request_password(
    mut commands: Commands,
    mut messages: EventReader<MessageReceived>,
    mut sender: EventWriter<SendMessage>,
    query: Query<(), (Without<Username>, Without<LoggedIn>)>,
) {
    for message in messages.read() {
        let conn = message.connection;
        if query.contains(conn) {
            let username = message.to_text();

            commands.entity(conn).insert(Username(username.clone()));

            if username.to_lowercase() == "new" {
                commands.entity(conn).insert(CharCreationState::default());
                println!("Inserting CharCreationState");
                return;
            }

            sender.echo(conn, false);
            sender.print(conn, "Password: ");
            sender.ga(conn);
        }
    }
}

fn login(
    mut commands: Commands,
    mut messages: EventReader<MessageReceived>,
    mut sender: EventWriter<SendMessage>,
    query: Query<&Username, (Without<LoggedIn>, Without<CharCreationState>)>,
) {
    for message in messages.read() {
        let conn = message.connection;
        if let Ok(Username(username)) = query.get(conn) {
            let password = message.to_text();

            sender.echo(conn, true);

            if password != "123456" {
                sender.println(conn, "Invalid password.");
                sender.print(conn, "Username: ");
                sender.ga(conn);
                commands.entity(conn).remove::<Username>();
                return;
            }

            commands.entity(conn).insert(LoggedIn);
            sender.println(conn, "");
            sender.println(conn, &format!("Logged in. Welcome {username}!"));
        }
    }
}
