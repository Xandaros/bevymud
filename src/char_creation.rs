use bevy::prelude::*;

use crate::{
    auth::Username,
    char::{Class, Race},
    database::DatabaseCommandsEx,
    telnet::{EventWriterTelnetEx, MessageReceived, SendMessage},
    util::EntityCommandsEx,
};

pub struct CharCreationPlugin;

impl Plugin for CharCreationPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                intro,
                choose_username,
                choose_password,
                choose_race,
                choose_class,
                show_menu,
            ),
        );
    }
}

#[derive(Component, Debug, Default, PartialEq, Eq)]
#[require(CharCreation)]
pub enum CharCreationState {
    #[default]
    Intro,
    Username,
    Password,
    Race,
    Class,
    Menu,
}

#[derive(Component, Debug, Default)]
struct CharCreation {
    pub username: Option<Username>,
    pub password: Option<String>,
    pub race: Option<Race>,
    pub class: Option<Class>,
}

impl CharCreation {
    fn is_complete(&self) -> bool {
        self.username.is_some()
            && self.password.is_some()
            && self.race.is_some()
            && self.class.is_some()
    }
}

fn intro(
    mut query: Query<(Entity, &mut CharCreationState), Added<CharCreationState>>,
    mut sender: EventWriter<SendMessage>,
) {
    for (ent, mut state) in &mut query {
        sender.println(ent, "Welcome!");
        *state = CharCreationState::Username;
    }
}

fn choose_username(
    mut commands: Commands,
    mut query: Query<(Entity, &mut CharCreationState), Changed<CharCreationState>>,
    mut sender: EventWriter<SendMessage>,
) {
    for (ent, state) in &mut query {
        if *state != CharCreationState::Username {
            continue;
        }

        sender.println(ent, "");
        sender.println(ent, "First, choose a username.");
        sender.print(ent, "Username: ");
        sender.ga(ent);

        commands.entity(ent).observe(
            |trigger: Trigger<MessageReceived>,
             mut commands: Commands,
             mut query: Query<(&mut CharCreation, &mut CharCreationState)>| {
                if let Ok((mut char, mut state)) = query.get_mut(trigger.target()) {
                    char.username = Some(Username(trigger.event().to_text()));
                    if char.is_complete() {
                        *state = CharCreationState::Menu
                    } else {
                        *state = CharCreationState::Password
                    }
                }
                commands.entity(trigger.observer()).despawn();
            },
        );
    }
}

fn choose_password(
    mut commands: Commands,
    query: Query<(Entity, &CharCreationState), Changed<CharCreationState>>,
    mut sender: EventWriter<SendMessage>,
) {
    for (ent, state) in &query {
        if *state != CharCreationState::Password {
            continue;
        }

        sender.println(ent, "");
        sender.print(ent, "Password: ");
        sender.echo(ent, false);
        sender.ga(ent);

        commands.entity(ent).observe_once(
            |trigger: Trigger<MessageReceived>,
             mut commands: Commands,
             mut sender: EventWriter<SendMessage>| {
                sender.println(trigger.target(), "");
                sender.print(trigger.target(), "Confirm password: ");
                sender.ga(trigger.target());

                let password = trigger.event().to_text();

                commands.entity(trigger.target()).observe_once(
                    move |trigger: Trigger<MessageReceived>,
                          mut query: Query<(&mut CharCreation, &mut CharCreationState)>,
                          mut sender: EventWriter<SendMessage>| {
                        let confirm = trigger.event().to_text();

                        if let Ok((mut char, mut state)) = query.get_mut(trigger.target()) {
                            if password != confirm {
                                sender.println(trigger.target(), "");
                                sender.println(trigger.target(), "Passwords do not match.");
                                *state = CharCreationState::Password;
                            } else {
                                char.password = Some(password.clone());
                                if char.is_complete() {
                                    *state = CharCreationState::Menu;
                                } else {
                                    *state = CharCreationState::Race;
                                }
                                sender.println(trigger.target(), "");
                                sender.echo(trigger.target(), true);
                            }
                        }
                    },
                );
            },
        );
    }
}

fn choose_race(
    mut commands: Commands,
    query: Query<(Entity, &CharCreationState), Changed<CharCreationState>>,
    mut sender: EventWriter<SendMessage>,
) {
    for (ent, state) in &query {
        if *state != CharCreationState::Race {
            continue;
        }

        sender.println(ent, "");
        sender.println(ent, "You must now choose a race.");
        sender.println(ent, "The following options are available:");
        sender.println(ent, "- Human\r\n- Elf\r\n- Orc\r\n- Pixie");
        sender.print(ent, "Race: ");
        sender.ga(ent);

        commands.entity(ent).observe_once(
            move |trigger: Trigger<MessageReceived>,
                  mut query: Query<(&mut CharCreation, &mut CharCreationState)>| {
                debug!("Received race input");
                let input = trigger.event().to_text();
                let race = match input.to_lowercase().as_str() {
                    "human" => Race::Human,
                    "elf" => Race::Elf,
                    "orc" => Race::Orc,
                    "pixie" => Race::Pixie,
                    _ => {
                        trace!("Invalid race");
                        if let Ok((_, mut state)) = query.get_mut(ent) {
                            *state = CharCreationState::Race;
                        }
                        return;
                    }
                };
                if let Ok((mut char, mut state)) = query.get_mut(ent) {
                    char.race = Some(race);
                    if char.is_complete() {
                        *state = CharCreationState::Menu;
                    } else {
                        *state = CharCreationState::Class;
                    }
                }
            },
        );
    }
}

fn choose_class(
    mut commands: Commands,
    query: Query<(Entity, &CharCreationState), Changed<CharCreationState>>,
    mut sender: EventWriter<SendMessage>,
) {
    for (ent, state) in &query {
        if *state != CharCreationState::Class {
            continue;
        }

        sender.println(ent, "");
        sender.println(ent, "You must now choose a class.");
        sender.println(ent, "The following options are available:");
        sender.println(ent, "- Warrior\r\n- Mage\r\n- Cleric");
        sender.print(ent, "Class: ");
        sender.ga(ent);

        commands.entity(ent).observe_once(
            move |trigger: Trigger<MessageReceived>,
                  mut query: Query<(&mut CharCreation, &mut CharCreationState)>| {
                let input = trigger.event().to_text();
                let class = match input.to_lowercase().as_str() {
                    "warrior" => Class::Warrior,
                    "mage" => Class::Mage,
                    "cleric" => Class::Cleric,
                    _ => {
                        if let Ok((_, mut state)) = query.get_mut(ent) {
                            *state = CharCreationState::Class;
                        }
                        return;
                    }
                };
                if let Ok((mut char, mut state)) = query.get_mut(ent) {
                    char.class = Some(class);
                    *state = CharCreationState::Menu;
                }
            },
        );
    }
}

fn show_menu(
    mut commands: Commands,
    query: Query<(Entity, &CharCreation, &CharCreationState), Changed<CharCreationState>>,
    mut sender: EventWriter<SendMessage>,
) -> Result {
    for (ent, chr, state) in &query {
        if *state != CharCreationState::Menu {
            continue;
        }

        let username = chr
            .username
            .as_ref()
            .map(Username::as_str)
            .ok_or("No username on char creation menu")?;

        let password = chr
            .password
            .as_ref()
            .ok_or("No password on char creation menu")?;

        let race = chr
            .race
            .as_ref()
            .map(Race::as_str)
            .ok_or("No race on char creation menu")?;

        let class = chr
            .class
            .as_ref()
            .map(Class::as_str)
            .ok_or("No class on char creation menu")?;

        sender.println(ent, "");
        sender.println(ent, &format!("Username: {}", username));
        sender.println(ent, "Password: ***");
        sender.println(ent, &format!("Race: {}", race));
        sender.println(ent, &format!("Class: {}", class));

        sender.println(ent, "");
        sender.println(ent, "Enter CONFIRM to continue, USERNAME, PASSWORD, RACE, or CLASS to edit the respective field.");
        sender.ga(ent);

        commands.entity(ent).observe_once(
            move |trigger: Trigger<MessageReceived>,
                  mut commands: Commands,
                  mut query: Query<(&mut CharCreationState, &CharCreation)>| -> Result {
                if let Ok((mut state, chr)) = query.get_mut(ent) {
                    let input = trigger.event().to_text();
                    match input.to_lowercase().as_str() {
                        "username" => *state = CharCreationState::Username,
                        "password" => *state = CharCreationState::Password,
                        "race" => *state = CharCreationState::Race,
                        "class" => *state = CharCreationState::Class,
                        "confirm" => {
                            let username = chr
                                .username
                                .as_ref()
                                .map(Username::as_str)
                                .ok_or("No username on char creation menu")?
                                .to_string();

                            let password = chr
                                .password
                                .as_ref()
                                .ok_or("No password on char creation menu")?
                                .to_string();

                            let race = chr
                                .race
                                .as_ref()
                                .map(Race::as_str)
                                .ok_or("No race on char creation menu")?
                                .to_string();

                            let class = chr
                                .class
                                .as_ref()
                                .map(Class::as_str)
                                .ok_or("No class on char creation menu")?
                                .to_string();
                            commands.run_sql(async move |pool| {
                                let hash = bcrypt::hash(password, bcrypt::DEFAULT_COST)?;
                                let result = sqlx::query("INSERT INTO users (username, password, race, class) VALUES (?, ?, ?, ?)")
                                    .bind(username)
                                    .bind(hash)
                                    .bind(race)
                                    .bind(class)
                                    .execute(&pool).await;
                                if let Err(err) = result {
                                    warn!("Character creation failed: {}", err);
                                }
                                Ok(())
                            }, |_: In<()>| {});
                        }
                        _ => *state = CharCreationState::Menu,
                    }
                }
                Ok(())
            },
        );
    }
    Ok(())
}
