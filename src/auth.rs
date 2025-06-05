use std::fmt::Display;

use bevy::prelude::*;
use bevy_yarnspinner::{events::ExecuteCommandEvent, prelude::*};

use crate::{
    database::DatabaseCommandsEx,
    menu::{EnterMenu, MenuLibrary},
    telnet::NewConnection,
};

pub struct AuthPlugin;

impl Plugin for AuthPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Username>()
            .register_type::<LoggedIn>()
            .add_systems(Startup, register_library_functions)
            .add_systems(Update, start_login)
            .add_systems(Update, on_login_command.after(YarnSpinnerSystemSet));
    }
}

fn register_library_functions(mut commands: Commands, mut library: ResMut<MenuLibrary>) {
    library.add_function("is_logged_in", commands.register_system(is_logged_in));
}

fn is_logged_in(In((entity, ())): In<(Entity, ())>, query: Query<(), With<LoggedIn>>) -> bool {
    query.contains(entity)
}

fn start_login(mut commands: Commands, mut conns: EventReader<NewConnection>) {
    for player in conns.read() {
        commands
            .entity(player.entity)
            .trigger(EnterMenu("System_Login_Start".to_string()));
    }
}

fn on_login_command(
    mut events: EventReader<ExecuteCommandEvent>,
    mut commands: Commands,
) -> Result {
    for event in events.read() {
        if event.command.name != "login" {
            return Ok(());
        }

        let username: String = (&event.command.parameters[0]).into();
        let password: String = (&event.command.parameters[1]).into();
        let conn = event.source;

        commands.run_sql(
            async move |pool| {
                let (hash,): (Vec<u8>,) =
                    sqlx::query_as("SELECT password FROM users WHERE username = ?")
                        .bind(&username)
                        .fetch_one(&pool)
                        .await?;
                let hash = String::from_utf8(hash)?;
                Ok((username, password, hash, conn))
            },
            |In((username, password, hash, conn)): In<(String, String, String, Entity)>,
             mut commands: Commands| {
                if let Ok(true) = bcrypt::verify(password, &hash) {
                    commands.entity(conn).insert((LoggedIn, Username(username)));
                }
            },
        );
    }
    Ok(())
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
