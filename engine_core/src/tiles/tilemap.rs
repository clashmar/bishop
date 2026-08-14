use crate::assets::sprite_manager::SpriteManager;
use crate::ecs::{CurrentFrame, Ecs, Pivot, Sprite, TilePlacement};
use crate::rendering::{EntityDrawParams, Renderable, RoomCompositionContext};
use crate::tiles::TileRegistry;
use crate::worlds::RoomLayer;
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
}

pub fn draw_room_tile_placements<C: BishopContext>(
    ctx: &mut C,
    ecs: &Ecs,
    layer: RoomLayer,
    composition: &RoomCompositionContext,
    tile_registry: &TileRegistry,
    sprite_manager: &mut SpriteManager,
) {
    let room_id = composition.room_id;
    let room_position = composition.room_position;
    let grid_size = composition.grid_size;

    for &entity in ecs.tile_entities_in_room_layer(room_id, layer).values() {
        let Some(tile) = ecs.get::<TilePlacement>(entity) else {
            continue;
        };
        let Some(tile_def) = tile_registry.get(tile.definition) else {
            continue;
        };

        let tile_pos = Vec2::new(tile.grid_x as f32 * grid_size, tile.grid_y as f32 * grid_size)
            + room_position;
        let tile_bounds = Rect::new(tile_pos.x, tile_pos.y, grid_size, grid_size);
        let color = if layer == RoomLayer::Front {
            let Some(color) = composition
                .front_layer_composition(ecs, entity, Some(tile_bounds))
                .tint()
            else {
                continue;
            };
            color
        } else {
            if !composition.back_layer_bounds_visible(tile_bounds) {
                continue;
            }
            Color::WHITE
        };
        let params = EntityDrawParams {
            pos: tile_pos,
            pivot: Pivot::TopLeft,
            grid_size,
            color,
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
            color,
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
    use crate::worlds::room::RoomVariant;
    use crate::worlds::RoomId;

    #[test]
    fn room_variant_background_is_read_from_room_variant_not_tilemap() {
        let variant = RoomVariant {
            background: Color::MAGENTA,
            tilemap: TileMap {
                background: Color::GREEN,
                ..TileMap::new(4, 4)
            },
            ..Default::default()
        };

        assert_eq!(variant.background, Color::MAGENTA);
        assert_eq!(variant.tilemap.background, Color::GREEN);
    }

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

    #[test]
    fn room_tile_draw_entities_when_layer_is_back_then_returns_only_back_tiles() {
        let mut ecs = Ecs::default();
        let room_id = RoomId(1);
        let front_tile = ecs.create_entity().finish();
        let back_tile = ecs.create_entity().finish();

        ecs.insert_component(front_tile, TilePlacement::new(TileDefId(1), 2, 3));
        ecs.insert_component(front_tile, CurrentRoom::front(room_id));
        ecs.insert_component(back_tile, TilePlacement::new(TileDefId(2), 4, 5));
        ecs.insert_component(back_tile, CurrentRoom::new(room_id, RoomLayer::Back));

        let draw_entities: Vec<_> = ecs
            .tile_entities_in_room_layer(room_id, RoomLayer::Back)
            .values()
            .copied()
            .collect();

        assert_eq!(draw_entities, vec![back_tile]);
    }
}
