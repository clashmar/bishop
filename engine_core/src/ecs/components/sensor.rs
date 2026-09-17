use super::Collider;
use crate::inspector_module;
use ecs_component::ecs_component;
use reflect_derive::Reflect;
use serde::{Deserialize, Serialize};

/// Marker for non-blocking physics overlap zones.
#[ecs_component(deps = [Collider])]
#[derive(Default, Clone, Copy, Serialize, Deserialize, Reflect)]
pub struct Sensor;

inspector_module!(Sensor, removable = true, title = "Sensor");
