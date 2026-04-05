// engine_core/src/assets/sprite.rs
use crate::assets::asset_manager::AssetManager;
use crate::ecs::entity::Entity;
use crate::game::EngineCtxMut;
use crate::inspector_module;
use crate::rendering::render_room::pivot_adjusted_position;
use crate::rendering::renderable::{EntityDrawParams, Renderable};
use bishop::prelude::*;
use ecs_component::ecs_component;
use reflect_derive::Reflect;
use serde::{Deserialize, Serialize};

/// Opaque handle that the asset manager gives out. Default/Unset is 0.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, Ord, PartialOrd, Hash, Serialize, Deserialize, Default,
)]
pub struct SpriteId(pub usize);

#[ecs_component(post_create = post_create, post_remove = post_remove)]
#[derive(Clone, Serialize, Deserialize, Reflect)]
pub struct Sprite {
    /// Reference to the texture stored by the AssetManager.
    pub sprite: SpriteId,
}

impl Default for Sprite {
    fn default() -> Self {
        Self {
            sprite: SpriteId(0),
        }
    }
}

fn post_create(sprite: &mut Sprite, _entity: &Entity, ctx: &mut dyn EngineCtxMut) {
    ctx.asset_manager().increment_ref(sprite.sprite);
}

fn post_remove(sprite: &mut Sprite, _entity: &Entity, ctx: &mut dyn EngineCtxMut) {
    ctx.asset_manager().decrement_ref(sprite.sprite);
}

inspector_module!(Sprite);

impl Renderable for Sprite {
    fn dimensions(&self, asset_manager: &AssetManager) -> Option<Vec2> {
        asset_manager
            .texture_size(self.sprite)
            .map(|(w, h)| vec2(w, h))
    }

    fn draw<C: BishopContext>(
        &self,
        ctx: &mut C,
        asset_manager: &mut AssetManager,
        params: &EntityDrawParams,
    ) -> bool {
        if self.sprite.0 == 0 {
            return false;
        }

        let tex = asset_manager.get_texture_from_id(ctx, self.sprite);
        let size = vec2(tex.width(), tex.height());
        let draw_base = pivot_adjusted_position(params.pos, size, params.pivot);
        ctx.draw_texture_ex(
            tex,
            draw_base.x,
            draw_base.y,
            Color::WHITE,
            DrawTextureParams {
                dest_size: Some(size),
                ..Default::default()
            },
        );
        true
    }
}
