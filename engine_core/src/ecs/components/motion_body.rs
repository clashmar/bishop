use super::SubPixel;
use crate::inspector_module;
use ecs_component::ecs_component;
use reflect_derive::Reflect;
use serde::{Deserialize, Serialize};

/// Marker for entities that participate in fixed-step movement.
#[ecs_component(lua_api = false, deps = [SubPixel])]
#[derive(Clone, Copy, Serialize, Deserialize, Default, Reflect)]
pub struct MotionBody;

inspector_module!(MotionBody, removable = true, title = "Motion Body");
