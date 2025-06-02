use bevy::{ecs::system::IntoObserverSystem, prelude::*};

pub trait EntityCommandsEx {
    fn observe_once<E, B, M>(&mut self, system: impl IntoObserverSystem<E, B, M>) -> &mut Self
    where
        E: Event,
        B: Bundle;
}

impl EntityCommandsEx for EntityCommands<'_> {
    fn observe_once<E, B, M>(&mut self, system: impl IntoObserverSystem<E, B, M>) -> &mut Self
    where
        E: Event,
        B: Bundle,
    {
        let ent_id = self.id();
        let mut commands = self.commands();

        let mut obs_ent = commands.spawn_empty();
        let obs_id = obs_ent.id();

        let sys = IntoObserverSystem::into_system(system);
        let mut observer = Observer::new(sys.pipe(
            move |_: In<Result>, mut commands: Commands| -> Result {
                commands.entity(obs_id).despawn();
                Ok(())
            },
        ));
        observer = observer.with_entity(ent_id);
        obs_ent.insert(observer);

        self
    }
}

pub trait FutureEx<O> {
    async fn print_result(self) -> Result<O>;
}

impl<O, F: Future<Output = Result<O>>> FutureEx<O> for F {
    async fn print_result(self) -> Result<O> {
        match self.await {
            Err(err) => {
                print!("Task failed: {}", err);
                Err(err)
            }
            Ok(v) => Ok(v),
        }
    }
}
