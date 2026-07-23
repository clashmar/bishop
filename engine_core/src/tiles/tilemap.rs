use crate::assets::sprite_manager::SpriteManager;
use crate::ecs::{Ecs, TilePlacement};
use crate::tiles::TileRegistry;
use crate::worlds::RoomId;
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
    for &entity in ecs.entities_in_room(room_id) {
        let Some(tile) = ecs.get::<TilePlacement>(entity) else {
            continue;
        };
        let Some(tile_def) = tile_registry.get(tile.definition) else {
            continue;
        };

        let tile_pos = Vec2::new(tile.grid_x as f32 * grid_size, tile.grid_y as f32 * grid_size)
            + room_position;
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
