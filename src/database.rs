use bevy::{
    prelude::*,
    tasks::{IoTaskPool, Task, block_on},
};
use sqlx::{MySql, mysql::MySqlPoolOptions};

use crate::util::FutureEx;

pub struct DatabasePlugin {
    uri: String,
}

impl DatabasePlugin {
    pub fn new(uri: String) -> Self {
        DatabasePlugin { uri }
    }
}

impl Plugin for DatabasePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(DatabaseConfig {
            uri: self.uri.clone(),
        });
        app.add_systems(PreStartup, setup);
        app.add_systems(PreUpdate, sql_callbacks);
    }
}

#[derive(Resource)]
struct DatabaseConfig {
    uri: String,
}

#[derive(Resource, Deref)]
struct SqlPool(sqlx::Pool<MySql>);

#[derive(Component)]
struct SqlTask(Option<Box<dyn RunSqlCallback + Send + Sync + 'static>>);

struct SqlTaskInner<Out, CBOut> {
    task: Option<Task<Result<Out>>>,
    callback: Box<dyn System<In = In<Out>, Out = CBOut>>,
}

trait RunSqlCallback {
    fn run_sql_callback(&mut self, world: &mut World) -> bool;
    fn ready(&self) -> bool;
}

impl<Out, CBOut> RunSqlCallback for SqlTaskInner<Out, CBOut>
where
    Out: 'static,
    CBOut: 'static,
{
    fn run_sql_callback(&mut self, world: &mut World) -> bool {
        let Some(ref task) = self.task else {
            // Task is gone - despawn entity
            return true;
        };

        if task.is_finished() {
            let Ok(output) = block_on(self.task.take().unwrap()) else {
                // Failed, but done - despawn
                return true;
            };
            self.callback.initialize(world);
            self.callback.run(output, world);
            // Done - despawn
            true
        } else {
            // Still running
            false
        }
    }

    fn ready(&self) -> bool {
        let Some(ref task) = self.task else {
            // Task is gone - needs to be processed
            return true;
        };
        task.is_finished()
    }
}

impl<Out, CBOut> SqlTaskInner<Out, CBOut>
where
    Out: 'static,
{
    fn new<CBMarker>(
        task: Task<Result<Out>>,
        callback: impl IntoSystem<In<Out>, CBOut, CBMarker>,
    ) -> Self {
        Self {
            task: Some(task),
            callback: Box::new(IntoSystem::into_system(callback)),
        }
    }
}

async fn init_sqlx(uri: String) -> Result<sqlx::Pool<MySql>> {
    let pool = MySqlPoolOptions::new()
        .max_connections(5)
        .connect(&uri)
        .await?;

    Ok(pool)
}

fn setup(mut commands: Commands, config: Res<DatabaseConfig>) -> Result {
    let task = IoTaskPool::get().spawn(init_sqlx(config.uri.clone()).print_result());
    let pool = block_on(task)?;

    commands.insert_resource(SqlPool(pool));
    Ok(())
}

fn sql_callbacks(world: &mut World, task_query: &mut QueryState<(Entity, &mut SqlTask)>) {
    let entries: Vec<(Entity, Mut<SqlTask>)> = task_query.iter_mut(world).collect();

    let mut despawn = Vec::with_capacity(entries.len());
    let mut to_be_processed = Vec::with_capacity(entries.len());

    for (entity, mut task_wrapper) in entries {
        let Some(ref task) = task_wrapper.0 else {
            despawn.push(entity);
            continue;
        };
        if task.ready() {
            to_be_processed.push(task_wrapper.0.take().unwrap());
        }
    }

    for mut task in to_be_processed {
        task.run_sql_callback(world);
    }
}

pub trait DatabaseCommandsEx {
    fn run_sql<F, Fut, Out, CBOut, CBMarker>(
        &mut self,
        f: F,
        callback: impl IntoSystem<In<Out>, CBOut, CBMarker> + Send + Sync + 'static,
    ) where
        F: FnOnce(sqlx::Pool<MySql>) -> Fut,
        F: Send + Sync + 'static,
        Fut: Future<Output = Result<Out>> + Send + 'static,
        Out: Send + Sync + 'static,
        CBOut: Send + Sync + 'static,
        CBMarker: Send + Sync + 'static;
}

impl DatabaseCommandsEx for Commands<'_, '_> {
    fn run_sql<F, Fut, Out, CBOut, CBMarker>(
        &mut self,
        f: F,
        callback: impl IntoSystem<In<Out>, CBOut, CBMarker> + Send + Sync + 'static,
    ) where
        F: FnOnce(sqlx::Pool<MySql>) -> Fut,
        F: Send + Sync + 'static,
        Fut: Future<Output = Result<Out>> + Send + 'static,
        Out: Send + Sync + 'static,
        CBOut: Send + Sync + 'static,
        CBMarker: Send + Sync + 'static,
    {
        self.queue(move |world: &mut World| {
            let pool = world.resource::<SqlPool>();
            let fut = f(pool.0.clone());

            let task = IoTaskPool::get().spawn(async move {
                let output = fut.await;

                if let Err(ref err) = output {
                    warn!("SQL task failed: {}", err);
                }

                output
            });

            world.spawn(SqlTask(Some(Box::new(SqlTaskInner::new(task, callback)))));
        });
    }
}
