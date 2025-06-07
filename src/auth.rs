use std::{
    fmt::Display,
    sync::{Arc, RwLock},
};

use bevy::prelude::*;
use bevy_yarnspinner::{events::ExecuteCommandEvent, prelude::*};

use crate::{
    database::DatabaseCommandsEx,
    menu::{EnterMenu, MenuLibrary},
    telnet::{Connection, NewConnection},
};

pub struct AuthPlugin;

impl Plugin for AuthPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Username>()
            .register_type::<LoggedIn>()
            .add_systems(Startup, register_library_functions)
            .add_systems(Update, start_login)
            .add_systems(
                Update,
                (on_login_command, on_register_account_command).after(YarnSpinnerSystemSet),
            );
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
    mut query: Query<&mut DialogueRunner>,
) -> Result {
    for event in events.read() {
        if event.command.name != "login" {
            continue;
        }

        let username: String = (&event.command.parameters[0]).into();
        let password: String = (&event.command.parameters[1]).into();
        let conn = event.source;

        let finished = Arc::new(RwLock::new(false));
        let mut runner = query.get_mut(event.source)?;

        runner.add_command_task(Box::new(Arc::clone(&finished)));

        commands.run_sql(
            async move |pool| {
                let Ok((hash,)): Result<(Vec<u8>,), sqlx::Error> =
                    sqlx::query_as("SELECT password FROM users WHERE username = ?")
                        .bind(&username)
                        .fetch_one(&pool)
                        .await
                else {
                    return Ok((username, "".to_string(), "".to_string(), conn));
                };
                let hash = String::from_utf8(hash)?;
                Ok((username, password, hash, conn))
            },
            move |In((username, password, hash, conn)): In<(String, String, String, Entity)>,
                  mut commands: Commands|
                  -> Result {
                if hash != "" {
                    if let Ok(true) = bcrypt::verify(password, &hash) {
                        commands.entity(conn).insert((LoggedIn, Username(username)));
                    }
                }
                *finished.write().map_err(|_| "Poisoned RwLock")? = true;
                Ok(())
            },
        );
    }
    Ok(())
}

struct RegistrationResult {
    error: Option<RegistrationError>,
    entity: Entity,
    username: String,
}

enum RegistrationError {
    AccountExists,
    SQLError,
}

impl From<sqlx::Error> for RegistrationError {
    fn from(_err: sqlx::Error) -> Self {
        Self::SQLError
    }
}

fn on_register_account_command(
    mut events: EventReader<ExecuteCommandEvent>,
    mut commands: Commands,
    mut query: Query<&mut DialogueRunner>,
) -> Result {
    for event in events.read() {
        if event.command.name != "register_account" {
            continue;
        }

        let conn = event.source;
        let username = String::from(&event.command.parameters[0]);
        let password = String::from(&event.command.parameters[1]);

        let finished = Arc::new(RwLock::new(false));
        let mut runner = query.get_mut(event.source)?;
        runner.add_command_task(Box::new(Arc::clone(&finished)));

        commands.run_sql(
            async move |pool| {
                let exists: bool = match sqlx::query_scalar(
                    "SELECT EXISTS (SELECT 1 FROM users WHERE username = ?)",
                )
                .bind(&username)
                .fetch_one(&pool)
                .await
                {
                    Ok(x) => x,
                    Err(err) => {
                        return Ok(RegistrationResult {
                            error: Some(err.into()),
                            entity: conn,
                            username,
                        });
                    }
                };

                if exists {
                    return Ok(RegistrationResult {
                        error: Some(RegistrationError::AccountExists),
                        entity: conn,
                        username,
                    });
                }

                match sqlx::query("INSERT INTO users(username, password) VALUES(?, ?)")
                    .bind(&username)
                    .bind(bcrypt::hash(&password, bcrypt::DEFAULT_COST)?)
                    .execute(&pool)
                    .await
                {
                    Ok(_) => (),
                    Err(err) => {
                        return Ok(RegistrationResult {
                            error: Some(err.into()),
                            entity: conn,
                            username,
                        });
                    }
                }

                Ok(RegistrationResult {
                    error: None,
                    entity: conn,
                    username,
                })
            },
            move |result: In<RegistrationResult>,
                  mut commands: Commands,
                  mut query: Query<&mut DialogueRunner, With<Connection>>|
                  -> Result {
                let mut runner = query.get_mut(result.entity)?;

                match &result.error {
                    Some(RegistrationError::AccountExists) => {
                        runner.variable_storage_mut().set(
                            "$error".to_string(),
                            YarnValue::String("AccountExists".to_string()),
                        )?;
                    }
                    Some(RegistrationError::SQLError) => {
                        runner
                            .variable_storage_mut()
                            .set("$error".to_string(), YarnValue::String("Error".to_string()))?;
                    }
                    None => {
                        runner
                            .variable_storage_mut()
                            .set("$error".to_string(), YarnValue::String("".to_string()))?;

                        commands
                            .entity(result.entity)
                            .insert((Username(result.0.username), LoggedIn));
                    }
                }

                *finished.write().map_err(|_| "Poisoned RwLock")? = true;

                Ok(())
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
