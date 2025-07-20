//! Provides miscellaneous components and events that don't fit in any single other plugin
use std::borrow::Cow;

use bevy::prelude::*;

pub struct MiscPlugin;

impl Plugin for MiscPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Description>().register_type::<Id>();
    }
}

#[derive(Clone, Reflect, Debug, Component, Hash)]
pub struct Description(pub Cow<'static, str>);

impl Description {
    pub fn new(desc: impl Into<Cow<'static, str>>) -> Self {
        Self(desc.into())
    }
}

#[derive(Clone, Copy, Reflect, Debug, Component, Hash)]
pub struct Id(pub u64);
