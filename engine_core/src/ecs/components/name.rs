use crate::inspector_module;
use ecs_component::ecs_component;
use reflect_derive::Reflect;
use serde::{Deserialize, Serialize};
use std::ops::{Deref, DerefMut};

/// The human readable name of the entity.
#[ecs_component]
#[derive(Debug, Clone, Serialize, Deserialize, Default, Reflect)]
pub struct Name(pub String);
inspector_module!(Name, removable = false);

impl Deref for Name {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Name {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
