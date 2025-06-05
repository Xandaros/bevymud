use std::sync::{Arc, RwLock};

use bevy::prelude::*;
use bevy_yarnspinner::{
    events::{ExecuteCommandEvent, PresentLineEvent},
    prelude::*,
};

use crate::{
    telnet::{EventWriterTelnetEx, MessageReceived, SendMessage},
    util::When,
};

pub struct MenuPlugin;

impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<EnterMenu>()
            .insert_resource(MenuLibrary(YarnLibrary::new()))
            .add_observer(on_enter_menu)
            .add_observer(on_input)
            .add_systems(
                Update,
                (on_present_line, handle_input_command, handle_echo_command)
                    .after(YarnSpinnerSystemSet),
            );
    }
}

#[derive(Resource, Deref, DerefMut)]
pub struct MenuLibrary(YarnLibrary);

#[derive(Component)]
pub struct InMenu;

#[derive(Event, Clone, Reflect)]
pub struct EnterMenu(pub String);

fn on_enter_menu(
    trigger: Trigger<EnterMenu>,
    mut commands: Commands,
    project: When<Res<YarnProject>>,
    library: Res<MenuLibrary>,
) {
    let mut runner = project.create_dialogue_runner(&mut commands);
    runner.library_mut().import(library.clone());
    runner.start_node(&trigger.0);

    commands.entity(trigger.target()).insert((InMenu, runner));
}

fn on_present_line(
    mut events: EventReader<PresentLineEvent>,
    mut sender: EventWriter<SendMessage>,
    mut query: Query<&mut DialogueRunner, With<InMenu>>,
) -> Result {
    for event in events.read() {
        let Ok(mut runner) = query.get_mut(event.source) else {
            continue;
        };

        if event.line.metadata.iter().any(|x| x == "prompt") {
            sender.print(event.source, &event.line.text);
            sender.print(event.source, " ");
            sender.ga(event.source);
        } else {
            sender.println(event.source, &event.line.text);
        }
        if !runner.is_waiting_for_option_selection() {
            runner.continue_in_next_update();
        }
    }

    Ok(())
}

#[derive(Component)]
struct WaitingOnInput {
    var: String,
    indicator: Arc<RwLock<bool>>,
}

fn handle_echo_command(
    mut events: EventReader<ExecuteCommandEvent>,
    mut sender: EventWriter<SendMessage>,
) -> Result {
    for event in events.read() {
        if event.command.name != "echo" {
            return Ok(());
        }

        sender.echo(
            event.source,
            event
                .command
                .parameters
                .get(0)
                .ok_or("Echo required a parameter")?
                .try_into()?,
        );
    }

    Ok(())
}

fn handle_input_command(
    mut events: EventReader<ExecuteCommandEvent>,
    mut commands: Commands,
    mut query: Query<&mut DialogueRunner>,
) -> Result {
    for event in events.read() {
        if event.command.name != "input" {
            continue;
        }

        let YarnValue::String(ref var) = event.command.parameters[0] else {
            return Err("Argument to <<input>> has invalid type, string expected".into());
        };

        let mut runner = query.get_mut(event.source)?;
        let indicator = Arc::new(RwLock::new(false));
        runner.add_command_task(Box::new(Arc::clone(&indicator)));

        commands.entity(event.source).insert(WaitingOnInput {
            var: var.to_string(),
            indicator,
        });
    }
    Ok(())
}

fn on_input(
    trigger: Trigger<MessageReceived>,
    mut commands: Commands,
    mut query: Query<(&mut DialogueRunner, &WaitingOnInput), With<InMenu>>,
) -> Result {
    let Ok((mut runner, waiter)) = query.get_mut(trigger.target()) else {
        return Ok(());
    };

    {
        let mut writer = waiter.indicator.write().map_err(|_| "RwLock poisoned.")?;
        *writer = true;
    }

    let var = &waiter.var;
    runner
        .variable_storage_mut()
        .set(format!("${}", var), trigger.to_text().into())?;

    commands.entity(trigger.target()).remove::<WaitingOnInput>();

    Ok(())
}
