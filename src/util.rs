use bevy::{
    ecs::system::{IntoObserverSystem, SystemParam, SystemParamValidationError},
    prelude::*,
};

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

#[derive(Deref, DerefMut)]
pub struct When<'world, 'state, T: SystemParam> {
    param: <T as SystemParam>::Item<'world, 'state>,
}

unsafe impl<'a, 'b, T: SystemParam> SystemParam for When<'a, 'b, T> {
    type State = <T as SystemParam>::State;

    type Item<'world, 'state> = When<'world, 'state, T>;

    fn init_state(
        world: &mut World,
        system_meta: &mut bevy::ecs::system::SystemMeta,
    ) -> Self::State {
        T::init_state(world, system_meta)
    }

    #[inline]
    unsafe fn get_param<'world, 'state>(
        state: &'state mut Self::State,
        system_meta: &bevy::ecs::system::SystemMeta,
        world: bevy::ecs::world::unsafe_world_cell::UnsafeWorldCell<'world>,
        change_tick: bevy::ecs::component::Tick,
    ) -> Self::Item<'world, 'state> {
        When {
            param: unsafe { T::get_param(state, system_meta, world, change_tick) },
        }
    }

    unsafe fn new_archetype(
        state: &mut Self::State,
        archetype: &bevy::ecs::archetype::Archetype,
        system_meta: &mut bevy::ecs::system::SystemMeta,
    ) {
        unsafe { T::new_archetype(state, archetype, system_meta) }
    }

    fn apply(
        state: &mut Self::State,
        system_meta: &bevy::ecs::system::SystemMeta,
        world: &mut World,
    ) {
        T::apply(state, system_meta, world)
    }

    fn queue(
        state: &mut Self::State,
        system_meta: &bevy::ecs::system::SystemMeta,
        world: bevy::ecs::world::DeferredWorld,
    ) {
        T::queue(state, system_meta, world)
    }

    #[inline]
    unsafe fn validate_param(
        state: &Self::State,
        system_meta: &bevy::ecs::system::SystemMeta,
        world: bevy::ecs::world::unsafe_world_cell::UnsafeWorldCell,
    ) -> std::result::Result<(), bevy::ecs::system::SystemParamValidationError> {
        unsafe { T::validate_param(state, system_meta, world) }
            .map_err(|err| SystemParamValidationError::skipped::<Self>(err.message))
    }
}
