use bevy::prelude::*;

use crate::telnet::{EventWriterTelnetEx, MessageReceived, NewConnection, SendMessage};

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

#[derive(Component, Reflect)]
pub struct Username(String);

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
        if let Ok(()) = query.get(conn) {
            let username = message.to_text();

            commands.entity(conn).insert(Username(username));

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
    query: Query<&Username, Without<LoggedIn>>,
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
