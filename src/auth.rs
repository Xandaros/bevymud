use std::{
    fmt::Display,
    sync::{Arc, RwLock},
};

use bevy::prelude::*;
use bevy_yarnspinner::{events::ExecuteCommandEvent, prelude::*};

use crate::{
    class::Classes,
    database::DatabaseCommandsEx,
    menu::{EnterMenu, MenuLibrary},
    race::Races,
    telnet::{Connection, EventWriterTelnetEx, NewConnection, SendMessage},
};

pub struct AuthPlugin;

impl Plugin for AuthPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Username>()
            .register_type::<LoggedIn>()
            .add_event::<CharacterLoginEvent>()
            .add_systems(Startup, register_library_functions)
            .add_systems(Update, start_login)
            .add_systems(
                Update,
                (
                    on_login_command,
                    on_register_account_command,
                    on_print_char_selection_command,
                    on_choose_char_command,
                )
                    .after(YarnSpinnerSystemSet),
            );
    }
}

#[derive(Clone, Debug, Event)]
struct CharacterLoginEvent {
    pub name: String,
    pub account: u64,
    pub class: u64,
    pub race: u64,
    pub room: u64,
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
                let Ok((acc_id, hash)): Result<(u64, Vec<u8>), sqlx::Error> =
                    sqlx::query_as("SELECT id, password FROM users WHERE username = ?")
                        .bind(&username)
                        .fetch_one(&pool)
                        .await
                else {
                    return Ok((u64::MAX, username, "".to_string(), "".to_string(), conn));
                };
                let hash = String::from_utf8(hash)?;
                Ok((acc_id, username, password, hash, conn))
            },
            move |In((acc_id, username, password, hash, conn)): In<(
                u64,
                String,
                String,
                String,
                Entity,
            )>,
                  mut commands: Commands|
                  -> Result {
                if hash != "" {
                    if let Ok(true) = bcrypt::verify(password, &hash) {
                        commands
                            .entity(conn)
                            .insert((LoggedIn(acc_id), Username(username)));
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
    error: RegistrationError,
    entity: Entity,
    username: String,
}

enum RegistrationError {
    Success(u64),
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
                            error: err.into(),
                            entity: conn,
                            username,
                        });
                    }
                };

                if exists {
                    return Ok(RegistrationResult {
                        error: RegistrationError::AccountExists,
                        entity: conn,
                        username,
                    });
                }

                let acc_id = match sqlx::query("INSERT INTO users(username, password) VALUES(?, ?)")
                    .bind(&username)
                    .bind(bcrypt::hash(&password, bcrypt::DEFAULT_COST)?)
                    .execute(&pool)
                    .await
                {
                    Ok(res) => res.last_insert_id(),
                    Err(err) => {
                        return Ok(RegistrationResult {
                            error: err.into(),
                            entity: conn,
                            username,
                        });
                    }
                };

                Ok(RegistrationResult {
                    error: RegistrationError::Success(acc_id),
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
                    RegistrationError::AccountExists => {
                        runner.variable_storage_mut().set(
                            "$error".to_string(),
                            YarnValue::String("AccountExists".to_string()),
                        )?;
                    }
                    RegistrationError::SQLError => {
                        runner
                            .variable_storage_mut()
                            .set("$error".to_string(), YarnValue::String("Error".to_string()))?;
                    }
                    RegistrationError::Success(acc_id) => {
                        runner
                            .variable_storage_mut()
                            .set("$error".to_string(), YarnValue::String("".to_string()))?;

                        let acc_id = *acc_id;
                        commands
                            .entity(result.entity)
                            .insert((Username(result.0.username), LoggedIn(acc_id)));
                    }
                }

                *finished.write().map_err(|_| "Poisoned RwLock")? = true;

                Ok(())
            },
        );
    }
    Ok(())
}

fn on_choose_char_command(
    mut events: EventReader<ExecuteCommandEvent>,
    mut commands: Commands,
    mut query: Query<(&DialogueRunner, &LoggedIn)>,
) -> Result {
    for event in events.read() {
        if event.command.name != "choose_char" {
            continue;
        }

        let conn = event.source;

        let (runner, acc_id) = query.get(conn)?;
        let acc_id = acc_id.0;

        let Some(selection) = event.command.parameters.get(0) else {
            // TODO: No selection
            debug!("No selection");
            return Ok(());
        };

        let YarnValue::String(selection) = selection.clone() else {
            // TODO: Not a string???
            debug!("Not a string???");
            return Ok(());
        };

        let Ok(selection) = selection.trim().parse::<u32>() else {
            // TODO: Nor parseable
            debug!("Not parseable");
            return Ok(());
        };

        commands.run_sql(
            async move |pool| {
                Ok(
                    sqlx::query_as(
                        "SELECT name, race, class, room FROM characters WHERE account = ? ORDER BY id ASC LIMIT ?, 1",
                    )
                    .bind(acc_id)
                    .bind(selection.saturating_sub(1))
                    .fetch_one(&pool)
                    .await
                    .map_err(Into::<BevyError>::into)
                )
            },
            move |res: In<Result<(String, u64, u64, u64)>>, mut commands: Commands| {
                let Ok((ref name, race, class, room)) = *res else {
                    // TODO: Invalid character
                    debug!("Invalid character");
                    return;
                };

                commands.trigger(CharacterLoginEvent {
                    name: name.clone(),
                    account: acc_id,
                    class,
                    race,
                    room: room as u64,
                });
            },
        );
    }
    Ok(())
}

fn on_print_char_selection_command(
    mut events: EventReader<ExecuteCommandEvent>,
    mut commands: Commands,
    mut query: Query<(&mut DialogueRunner, &LoggedIn)>,
) -> Result {
    for event in events.read() {
        if event.command.name != "print_char_selection" {
            continue;
        }

        let conn = event.source;

        let (mut runner, acc_id) = query.get_mut(conn)?;

        let acc_id = acc_id.0;

        let finished = Arc::new(RwLock::new(false));
        let sql_finished = Arc::clone(&finished);
        runner.add_command_task(Box::new(Arc::clone(&finished)));

        commands.run_sql(
            async move |pool| {
                //let Ok(res): Result<Vec<(String, u64, u64)>, sqlx::Error> = sqlx::query_as(
                let res = match sqlx::query_as(
                    "SELECT name, race, class FROM characters WHERE account = ? ORDER BY id",
                )
                .bind(acc_id)
                .fetch_all(&pool)
                .await
                {
                    Err(err) => {
                        warn!("SQL query failed: {err}");
                        *sql_finished.write().map_err(|_| "Poisoned RwLock")? = true;
                        return Ok((conn, None));
                    }
                    Ok(res) => res,
                };
                Ok((conn, Some(res)))
            },
            move |In((conn, chars)): In<(Entity, Option<Vec<(String, u64, u64)>>)>,
                  classes: Res<Classes>,
                  races: Res<Races>,
                  mut sender: EventWriter<SendMessage>,
                  mut query: Query<&mut DialogueRunner, With<Connection>>|
                  -> Result {
                let mut runner = query.get_mut(conn)?;

                let Some(chars) = chars else {
                    runner
                        .variable_storage_mut()
                        .set("$error".to_string(), "Error".into())?;
                    *finished.write().map_err(|_| "Poisoned RwLock")? = true;
                    return Ok(());
                };
                runner
                    .variable_storage_mut()
                    .set("$error".to_string(), "".into())?;

                for (i, char) in chars.iter().enumerate() {
                    let name = &char.0;
                    let race = races.get_race(char.1);
                    let class = classes.get_class(char.2);

                    sender.print(conn, &format!("{}: ", i + 1));
                    sender.print(conn, name);
                    sender.print(conn, " - ");
                    sender.print(conn, &race.name);
                    sender.print(conn, " ");
                    sender.println(conn, &class.name);
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
pub struct LoggedIn(u64);
