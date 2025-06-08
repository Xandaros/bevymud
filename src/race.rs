use std::sync::LazyLock;

use bevy::prelude::*;

use crate::database::{self, DatabaseCommandsEx};

pub struct RacePlugin;

impl Plugin for RacePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Races>()
            .add_systems(PreStartup, load_races.after(database::DatabaseSystemSet));
    }
}

#[derive(sqlx::FromRow, Clone, Reflect)]
pub struct RaceDef {
    pub id: u64,
    pub name: String,
}

pub static PLACEHOLDER_RACEDEF: LazyLock<RaceDef> = LazyLock::new(|| RaceDef {
    id: u64::MAX,
    name: "Invalid race".to_string(),
});

#[derive(Resource, Default)]
pub struct Races(Vec<RaceDef>);

impl Races {
    pub fn get_race(&self, id: u64) -> &RaceDef {
        self.0
            .iter()
            .filter(|x| x.id == id)
            .next()
            .unwrap_or(&PLACEHOLDER_RACEDEF)
    }
}

fn load_races(mut commands: Commands) {
    commands.run_sql(
        async |pool| {
            let res = sqlx::query_as("SELECT * FROM races")
                .fetch_all(&pool)
                .await?;
            Ok(res)
        },
        |res: In<Vec<RaceDef>>, mut racees: ResMut<Races>| {
            racees.0 = res.clone();
        },
    );
}
