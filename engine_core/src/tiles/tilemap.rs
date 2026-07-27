use crate::assets::sprite_manager::SpriteManager;
use crate::ecs::{CurrentFrame, Ecs, Pivot, Sprite, TilePlacement};
use crate::rendering::{EntityDrawParams, Renderable};
use crate::tiles::TileRegistry;
use crate::worlds::{RoomId, RoomLayer};
use bishop::prelude::*;
use serde::{Deserialize, Serialize};
use serde_with::{FromInto, serde_as};

#[serde_as]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TileMap {
    pub width: usize,
    pub height: usize,
    #[serde_as(as = "FromInto<[f32; 4]>")]
    pub background: Color,
}

impl TileMap {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            background: Color::LIGHTGREY,
        }
    }

    pub fn draw_background<C: BishopContext>(
        &self,
        ctx: &mut C,
        room_position: Vec2,
        grid_size: f32,
    ) {
        ctx.draw_rectangle(
            room_position.x,
            room_position.y,
            self.width as f32 * grid_size,
            self.height as f32 * grid_size,
            self.background,
        );
    }
}

pub fn draw_room_tile_placements<C: BishopContext>(
    ctx: &mut C,
    ecs: &Ecs,
    room_id: RoomId,
    tile_registry: &TileRegistry,
    sprite_manager: &mut SpriteManager,
    room_position: Vec2,
    grid_size: f32,
) {
    for &entity in ecs
        .tile_entities_in_room_layer(room_id, RoomLayer::Front)
        .values()
    {
        let Some(tile) = ecs.get::<TilePlacement>(entity) else {
            continue;
        };
        let Some(tile_def) = tile_registry.get(tile.definition) else {
            continue;
        };

        let tile_pos = Vec2::new(tile.grid_x as f32 * grid_size, tile.grid_y as f32 * grid_size)
            + room_position;
        let params = EntityDrawParams {
            pos: tile_pos,
            pivot: Pivot::TopLeft,
            grid_size,
        };

        if let Some(current_frame) = ecs.get::<CurrentFrame>(entity)
            && current_frame.draw(ctx, sprite_manager, &params)
        {
            continue;
        }

        if let Some(sprite) = ecs.get::<Sprite>(entity)
            && sprite.draw(ctx, sprite_manager, &params)
        {
            continue;
        }

        let tex = sprite_manager.get_texture_from_id(ctx, tile_def.sprite_id);
        ctx.draw_texture_ex(
            tex,
            tile_pos.x,
            tile_pos.y,
            Color::WHITE,
            DrawTextureParams {
                dest_size: Some(Vec2::new(grid_size, grid_size)),
                ..Default::default()
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecs::CurrentRoom;
    use crate::tiles::TileDefId;

    #[test]
    fn room_tile_draw_entities_when_room_contains_non_tile_entity_then_returns_only_tiles() {
        let mut ecs = Ecs::default();
        let room_id = RoomId(1);
        let tile_entity = ecs.create_entity().finish();
        let non_tile_entity = ecs.create_entity().finish();

        ecs.insert_component(tile_entity, TilePlacement::new(TileDefId(1), 2, 3));
        ecs.insert_component(tile_entity, CurrentRoom::front(room_id));
        ecs.insert_component(non_tile_entity, CurrentRoom::front(room_id));

        ecs.room_entities.insert(room_id, std::iter::once(non_tile_entity).collect());

        let draw_entities: Vec<_> = ecs
            .tile_entities_in_room_layer(room_id, RoomLayer::Front)
            .values()
            .copied()
            .collect();

        assert_eq!(draw_entities, vec![tile_entity]);
    }
}
