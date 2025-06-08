use std::sync::LazyLock;

use bevy::prelude::*;

use crate::database::{self, DatabaseCommandsEx};

pub struct ClassPlugin;

impl Plugin for ClassPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Classes>()
            .add_systems(PreStartup, load_classes.after(database::DatabaseSystemSet));
    }
}

#[derive(sqlx::FromRow, Clone, Reflect)]
pub struct ClassDef {
    pub id: u64,
    pub name: String,
}

pub static PLACEHOLDER_CLASSDEF: LazyLock<ClassDef> = LazyLock::new(|| ClassDef {
    id: u64::MAX,
    name: "Invalid class".to_string(),
});

#[derive(Resource, Default)]
pub struct Classes(Vec<ClassDef>);

impl Classes {
    pub fn get_class(&self, id: u64) -> &ClassDef {
        self.0
            .iter()
            .filter(|x| x.id == id)
            .next()
            .unwrap_or(&PLACEHOLDER_CLASSDEF)
    }
}

fn load_classes(mut commands: Commands) {
    commands.run_sql(
        async |pool| {
            let res = sqlx::query_as("SELECT * FROM classes")
                .fetch_all(&pool)
                .await?;
            Ok(res)
        },
        |res: In<Vec<ClassDef>>, mut classes: ResMut<Classes>| {
            classes.0 = res.clone();
        },
    );
}
