// engine_core/src/tiles/tilemap.rs
use crate::assets::asset_manager::AssetManager;
use crate::tiles::serialization::{deserialize_tiles, serialize_tiles};
use crate::tiles::tile::TileDefId;
use crate::worlds::world::GridPos;
use bishop::prelude::*;
use serde::{Deserialize, Serialize};
use serde_with::{FromInto, serde_as};
use std::collections::HashMap;

#[serde_as]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TileMap {
    pub width: usize,
    pub height: usize,
    #[serde(
        default,
        skip_serializing_if = "HashMap::is_empty",
        serialize_with = "serialize_tiles",
        deserialize_with = "deserialize_tiles"
    )]
    pub tiles: HashMap<(usize, usize), TileDefId>,
    #[serde_as(as = "FromInto<[f32; 4]>")]
    pub background: Color,
}

impl TileMap {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            tiles: HashMap::new(),
            background: Color::LIGHTGREY,
        }
    }

    /// Draw the tilemap.
    pub fn draw<C: BishopContext>(
        &self,
        ctx: &mut C,
        asset_manager: &mut AssetManager,
        room_position: Vec2,
        grid_size: f32,
    ) {
        // Background
        ctx.draw_rectangle(
            room_position.x,
            room_position.y,
            self.width as f32 * grid_size,
            self.height as f32 * grid_size,
            self.background,
        );

        for ((x, y), tile_def_id) in &self.tiles {
            let tile_pos = Vec2::new(*x as f32 * grid_size, *y as f32 * grid_size) + room_position;

            if let Some(tile_def) = asset_manager.tile_defs.get(tile_def_id) {
                let tex = asset_manager.get_texture_from_id(ctx, tile_def.sprite_id);
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
    }

    /// Insert a tile at a grid coordinate.
    pub fn set_tile(&mut self, x: usize, y: usize, tile_def_id: TileDefId) {
        self.tiles.insert((x, y), tile_def_id);
    }

    /// Retrieve a tile, returning `None` for empty cells.
    pub fn get_tile(&self, pos: GridPos) -> Option<&TileDefId> {
        let (x, y) = pos.as_usize()?;
        self.tiles.get(&(x, y))
    }

    /// Convert a pixel coordinate to a grid coordinate.
    pub fn pixel_to_grid(pixel: f32, grid_size: f32) -> i32 {
        (pixel / grid_size).floor() as i32
    }

    pub fn any_tiles_in_range<F>(
        map: &TileMap,
        x_range: std::ops::RangeInclusive<i32>,
        y_range: std::ops::RangeInclusive<i32>,
        predicate: F,
    ) -> bool
    where
        F: Fn(&TileDefId) -> bool,
    {
        let y_start = *y_range.start();
        let y_end = *y_range.end();

        for x in x_range {
            for y in y_start..=y_end {
                let pos = GridPos::new(x, y);
                if pos.is_in_bounds(map.width, map.height)
                    && let Some(tile) = map.get_tile(pos)
                    && predicate(tile)
                {
                    return true;
                }
            }
        }
        false
    }

    /// Remove a tile from the map.
    pub fn remove_tile(&mut self, grid_position: (usize, usize)) {
        if let Some(_tile_def_id) = self.tiles.remove(&grid_position) {
            // TODO: Handle sprite and ecs
        }
    }
}

/// Convert a grid position to world coordinates.
pub fn tile_to_world(grid_position: GridPos, grid_size: f32) -> Vec2 {
    Vec2::new(
        grid_position.x() as f32 * grid_size,
        grid_position.y() as f32 * grid_size,
    )
}

/// Shift every tile in a tilemap by (dx, dy).
pub fn shift_tiles(map: &mut TileMap, dx: isize, dy: isize) {
    // Nothing to do if the offset is zero
    if dx == 0 && dy == 0 {
        return;
    }

    // Take the current tiles out of the map, then re‑insert them with the offset
    let old_tiles = std::mem::take(&mut map.tiles);

    for ((x, y), tile) in old_tiles {
        // New grid coordinates.
        let nx = (x as isize + dx) as usize;
        let ny = (y as isize + dy) as usize;

        // Re‑insert the tile at its new grid location
        map.tiles.insert((nx, ny), tile);
    }
}
