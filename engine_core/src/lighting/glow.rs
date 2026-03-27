// engine_core/src/lighting/glow.rs
use crate::assets::sprite::SpriteId;
use crate::ecs::entity::Entity;
use crate::game::GameCtxMut;
use crate::inspector_module;
use bishop::prelude::*;
use ecs_component::ecs_component;
use reflect_derive::Reflect;
use serde::{Deserialize, Serialize};
use serde_with::{FromInto, serde_as};

/// A single glow source.
#[ecs_component(post_create = post_create, post_remove = post_remove)]
#[serde_as]
#[derive(Clone, Serialize, Deserialize, Debug, Reflect)]
#[serde(default)]
pub struct Glow {
    #[serde_as(as = "FromInto<[f32; 3]>")]
    pub color: Vec3,
    pub intensity: f32,
    pub brightness: f32,
    pub emission: f32,
    #[widget("png")]
    pub sprite_id: SpriteId,
}

inspector_module!(Glow);

impl Default for Glow {
    fn default() -> Self {
        Self {
            color: vec3(1.0, 1.0, 1.0),
            intensity: 0.1,
            brightness: 0.5,
            emission: 0.0,
            sprite_id: SpriteId(0),
        }
    }
}

fn post_create(glow: &mut Glow, _entity: &Entity, ctx: &mut GameCtxMut) {
    ctx.asset_manager.increment_ref(glow.sprite_id);
}

fn post_remove(glow: &mut Glow, _entity: &Entity, ctx: &mut GameCtxMut) {
    ctx.asset_manager.decrement_ref(glow.sprite_id);
}
