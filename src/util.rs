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
        let mut observer = Observer::new(sys.pipe(move |_: In<()>, mut commands: Commands| {
            commands.entity(obs_id).despawn();
        }));
        observer = observer.with_entity(ent_id);
        obs_ent.insert(observer);

        self
    }
}
