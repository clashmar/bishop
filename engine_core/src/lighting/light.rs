// engine_core/src/lighting/light.rs
use crate::inspector_module;
use bishop::prelude::*;
use ecs_component::ecs_component;
use reflect_derive::Reflect;
use serde::{Deserialize, Serialize};
use serde_with::{FromInto, serde_as};

#[ecs_component]
#[serde_as]
#[derive(Clone, Copy, Serialize, Deserialize, Reflect)]
#[serde(default)]
pub struct Light {
    /// Relative to the entity the light is attached to.
    #[serde_as(as = "FromInto<[f32; 2]>")]
    pub pos: Vec2,
    #[serde_as(as = "FromInto<[f32; 3]>")]
    pub color: Vec3,
    /// Intensity of the color tint.
    pub intensity: f32,
    pub radius: f32,
    pub spread: f32,
    pub alpha: f32,
    pub brightness: f32,
}

inspector_module!(Light);

impl Default for Light {
    fn default() -> Self {
        Light {
            pos: Vec2::new(0., 0.),
            color: Vec3::new(1., 1., 1.),
            intensity: 0.5,
            radius: 50.,
            spread: 50.,
            alpha: 0.5,
            brightness: 1.,
        }
    }
}
