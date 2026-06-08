use super::*;
use crate::ecs::Transform;

use serde::{Deserialize, Serialize};

use ecs_component::ecs_component;

#[ecs_component(on_insert = lifecycle_on_insert, on_remove = lifecycle_on_remove, guarded)]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct LifecycleMarker {
    insert_count: u32,
    remove_count: u32,
}

fn lifecycle_on_insert(comp: &mut LifecycleMarker, _entity: &Entity, _ecs: &mut Ecs) {
    comp.insert_count += 1;
}

fn lifecycle_on_remove(comp: &mut LifecycleMarker, _entity: &Entity, _ecs: &mut Ecs) {
    comp.remove_count += 1;
}

mod ecs_tests;
