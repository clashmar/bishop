// editor/src/commands/room/resize_tilemap_cmd.rs
use crate::app::EditorMode;
use crate::commands::editor_command_manager::EditorCommand;
use crate::tilemap::resize_handle::HandleSide;
use crate::with_editor;
use engine_core::prelude::*;
use std::collections::HashMap;

/// Undoable command for resizing a tilemap via drag handles.
#[derive(Debug)]
pub struct ResizeTilemapCmd {
    room_id: RoomId,
    variant_index: usize,
    side: HandleSide,
    delta: i32,
    // Old state for undo
    old_width: usize,
    old_height: usize,
    old_position: Vec2,
    old_size: Vec2,
    old_tiles: HashMap<(usize, usize), TileDefId>,
    old_exits: Vec<Exit>,
    // Track if we've captured the old state
    state_captured: bool,
}

impl ResizeTilemapCmd {
    /// Create a new resize command.
    pub fn new(room_id: RoomId, variant_index: usize, side: HandleSide, delta: i32) -> Self {
        Self {
            room_id,
            variant_index,
            side,
            delta,
            old_width: 0,
            old_height: 0,
            old_position: Vec2::ZERO,
            old_size: Vec2::ZERO,
            old_tiles: HashMap::new(),
            old_exits: Vec::new(),
            state_captured: false,
        }
    }

    /// Capture the current state before making changes (for undo).
    fn capture_state(&mut self) {
        if self.state_captured {
            return;
        }

        with_editor(|editor| {
            let world = editor.game.current_world();
            if let Some(room) = world.rooms.iter().find(|r| r.id == self.room_id) {
                let map = &room.variants[self.variant_index].tilemap;
                self.old_width = map.width;
                self.old_height = map.height;
                self.old_position = room.position;
                self.old_size = room.size;
                self.old_tiles = map.tiles.clone();
                self.old_exits = room.exits.clone();
            }
        });

        self.state_captured = true;
    }
}

impl EditorCommand for ResizeTilemapCmd {
    fn execute(&mut self) {
        // Capture state on first execution
        self.capture_state();

        with_editor(|editor| {
            let grid_size = editor.game.current_world().grid_size;
            let room = editor
                .game
                .current_world_mut()
                .rooms
                .iter_mut()
                .find(|r| r.id == self.room_id);

            let room = match room {
                Some(r) => r,
                None => return,
            };

            let map = &mut room.variants[self.variant_index].tilemap;
            let room_position = &mut room.position;
            let room_size = &mut room.size;
            let exits = &mut room.exits;

            match self.side {
                HandleSide::Top => {
                    if self.delta > 0 {
                        // Expand top
                        map.height += self.delta as usize;
                        shift_tiles(map, 0, self.delta as isize);
                        for exit in exits.iter_mut() {
                            let on_bottom = (exit.position.y - room_size.y).abs() < f32::EPSILON;
                            let on_left = (exit.position.x + 1.0).abs() < f32::EPSILON;
                            let on_right = (exit.position.x - room_size.x).abs() < f32::EPSILON;
                            if on_bottom || on_left || on_right {
                                exit.position.y += self.delta as f32;
                            }
                        }
                        room_size.y += self.delta as f32;
                        room_position.y -= self.delta as f32 * grid_size;
                    } else if self.delta < 0 {
                        // Shrink top
                        let shrink = (-self.delta) as usize;
                        if map.height > shrink {
                            // Remove tiles in top rows
                            for dy in 0..shrink {
                                for x in 0..map.width {
                                    map.tiles.remove(&(x, dy));
                                }
                            }
                            map.height -= shrink;
                            shift_tiles(map, 0, -(shrink as isize));
                            for exit in exits.iter_mut() {
                                let on_bottom =
                                    (exit.position.y - room_size.y).abs() < f32::EPSILON;
                                let on_left = (exit.position.x + 1.0).abs() < f32::EPSILON;
                                let on_right = (exit.position.x - room_size.x).abs() < f32::EPSILON;
                                if on_bottom || on_left || on_right {
                                    exit.position.y -= shrink as f32;
                                }
                            }
                            room_size.y -= shrink as f32;
                            room_position.y += shrink as f32 * grid_size;
                        }
                    }
                }
                HandleSide::Bottom => {
                    if self.delta > 0 {
                        // Expand bottom
                        map.height += self.delta as usize;
                        for exit in exits.iter_mut() {
                            if (exit.position.y - room_size.y).abs() < f32::EPSILON {
                                exit.position.y += self.delta as f32;
                            }
                        }
                        room_size.y += self.delta as f32;
                    } else if self.delta < 0 {
                        // Shrink bottom
                        let shrink = (-self.delta) as usize;
                        if map.height > shrink {
                            // Remove tiles in bottom rows
                            for dy in 0..shrink {
                                let y = map.height - 1 - dy;
                                for x in 0..map.width {
                                    map.tiles.remove(&(x, y));
                                }
                            }
                            map.height -= shrink;
                            for exit in exits.iter_mut() {
                                if (exit.position.y - room_size.y).abs() < f32::EPSILON {
                                    exit.position.y -= shrink as f32;
                                }
                            }
                            room_size.y -= shrink as f32;
                        }
                    }
                }
                HandleSide::Left => {
                    if self.delta > 0 {
                        // Expand left
                        map.width += self.delta as usize;
                        shift_tiles(map, self.delta as isize, 0);
                        for exit in exits.iter_mut() {
                            let on_right = (exit.position.x - room_size.x).abs() < f32::EPSILON;
                            let on_top = (exit.position.y + 1.0).abs() < f32::EPSILON;
                            let on_bottom = (exit.position.y - room_size.y).abs() < f32::EPSILON;
                            if on_right || on_top || on_bottom {
                                exit.position.x += self.delta as f32;
                            }
                        }
                        room_size.x += self.delta as f32;
                        room_position.x -= self.delta as f32 * grid_size;
                    } else if self.delta < 0 {
                        // Shrink left
                        let shrink = (-self.delta) as usize;
                        if map.width > shrink {
                            // Remove tiles in left columns
                            for dx in 0..shrink {
                                for y in 0..map.height {
                                    map.tiles.remove(&(dx, y));
                                }
                            }
                            map.width -= shrink;
                            shift_tiles(map, -(shrink as isize), 0);
                            for exit in exits.iter_mut() {
                                let on_right = (exit.position.x - room_size.x).abs() < f32::EPSILON;
                                let on_top = (exit.position.y + 1.0).abs() < f32::EPSILON;
                                let on_bottom =
                                    (exit.position.y - room_size.y).abs() < f32::EPSILON;
                                if on_right || on_top || on_bottom {
                                    exit.position.x -= shrink as f32;
                                }
                            }
                            room_size.x -= shrink as f32;
                            room_position.x += shrink as f32 * grid_size;
                        }
                    }
                }
                HandleSide::Right => {
                    if self.delta > 0 {
                        // Expand right
                        map.width += self.delta as usize;
                        for exit in exits.iter_mut() {
                            if (exit.position.x - room_size.x).abs() < f32::EPSILON {
                                exit.position.x += self.delta as f32;
                            }
                        }
                        room_size.x += self.delta as f32;
                    } else if self.delta < 0 {
                        // Shrink right
                        let shrink = (-self.delta) as usize;
                        if map.width > shrink {
                            // Remove tiles in right columns
                            for dx in 0..shrink {
                                let x = map.width - 1 - dx;
                                for y in 0..map.height {
                                    map.tiles.remove(&(x, y));
                                }
                            }
                            map.width -= shrink;
                            for exit in exits.iter_mut() {
                                if (exit.position.x - room_size.x).abs() < f32::EPSILON {
                                    exit.position.x -= shrink as f32;
                                }
                            }
                            room_size.x -= shrink as f32;
                        }
                    }
                }
            }
        });
    }

    fn undo(&mut self) {
        with_editor(|editor| {
            let room = editor
                .game
                .current_world_mut()
                .rooms
                .iter_mut()
                .find(|r| r.id == self.room_id);

            let room = match room {
                Some(r) => r,
                None => return,
            };

            // Restore all captured state
            room.position = self.old_position;
            room.size = self.old_size;
            room.exits = self.old_exits.clone();

            let map = &mut room.variants[self.variant_index].tilemap;
            map.width = self.old_width;
            map.height = self.old_height;
            map.tiles = self.old_tiles.clone();
        });
    }

    fn mode(&self) -> EditorMode {
        EditorMode::Room(self.room_id)
    }
}
