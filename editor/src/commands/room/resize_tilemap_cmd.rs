use bishop::prelude::*;
use crate::app::EditorMode;
use crate::commands::editor_command_manager::EditorCommand;
use crate::tilemap::resize_handle::HandleSide;
use crate::with_editor;
use engine_core::ecs::{Ecs, TilePlacement};
use engine_core::game::GameCtxMut;
use engine_core::tiles::apply_tile_placement_definition;
use engine_core::worlds::*;

/// Undoable command for resizing a tilemap via drag handles.
#[derive(Debug)]
pub struct ResizeTilemapCmd {
    room_id: RoomId,
    variant_index: usize,
    side: HandleSide,
    delta: i32,
    old_width: usize,
    old_height: usize,
    old_position: Vec2,
    old_size: Vec2,
    old_placements: Vec<TilePlacement>,
    old_exits: Vec<Exit>,
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
            old_placements: Vec::new(),
            old_exits: Vec::new(),
            state_captured: false,
        }
    }

    /// Capture the current state before making changes for undo.
    fn capture_state(&mut self) {
        if self.state_captured {
            return;
        }

        with_editor(|editor| {
            let world = editor.game.current_world();
            if let Some(room) = world.get_room(self.room_id) {
                let map = &room.variants[self.variant_index].tilemap;
                self.old_width = map.width;
                self.old_height = map.height;
                self.old_position = room.position;
                self.old_size = room.size;
                self.old_placements = room_tile_placements(&editor.game.ecs, self.room_id);
                self.old_exits = room.exits.clone();
            }
        });

        self.state_captured = true;
    }
}

impl EditorCommand for ResizeTilemapCmd {
    fn execute(&mut self) {
        self.capture_state();
        let next_placements = resized_placements(
            &self.old_placements,
            self.side,
            self.delta,
            self.old_width,
            self.old_height,
        );

        with_editor(|editor| {
            let grid_size = editor.game.current_world().grid_size;

            {
                let Some(current_world) = editor.game.current_world_mut() else {
                    return;
                };
                let Some(room) = current_world
                    .rooms_mut()
                    .iter_mut()
                    .find(|room| room.id == self.room_id)
                else {
                    return;
                };

                let map = &mut room.variants[self.variant_index].tilemap;
                let room_position = &mut room.position;
                let room_size = &mut room.size;
                let exits = &mut room.exits;

                match self.side {
                    HandleSide::Top => {
                        if self.delta > 0 {
                            map.height += self.delta as usize;
                            for exit in exits.iter_mut() {
                                let on_top = (exit.position.y + 1.0).abs() < f32::EPSILON;
                                let on_bottom = (exit.position.y - room_size.y).abs() < f32::EPSILON;
                                if on_top {
                                    exit.position.y -= self.delta as f32;
                                } else if on_bottom {
                                    exit.position.y += self.delta as f32;
                                }
                            }
                            room_size.y += self.delta as f32;
                            room_position.y -= self.delta as f32 * grid_size;
                        } else if self.delta < 0 {
                            let shrink = (-self.delta) as usize;
                            if map.height > shrink {
                                map.height -= shrink;
                                for exit in exits.iter_mut() {
                                    let on_top = (exit.position.y + 1.0).abs() < f32::EPSILON;
                                    let on_bottom = (exit.position.y - room_size.y).abs() < f32::EPSILON;
                                    if on_top {
                                        exit.position.y += shrink as f32;
                                    } else if on_bottom {
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
                            map.height += self.delta as usize;
                            for exit in exits.iter_mut() {
                                if (exit.position.y - room_size.y).abs() < f32::EPSILON {
                                    exit.position.y += self.delta as f32;
                                }
                            }
                            room_size.y += self.delta as f32;
                        } else if self.delta < 0 {
                            let shrink = (-self.delta) as usize;
                            if map.height > shrink {
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
                            map.width += self.delta as usize;
                            for exit in exits.iter_mut() {
                                let on_left = (exit.position.x + 1.0).abs() < f32::EPSILON;
                                let on_right = (exit.position.x - room_size.x).abs() < f32::EPSILON;
                                if on_left {
                                    exit.position.x -= self.delta as f32;
                                } else if on_right {
                                    exit.position.x += self.delta as f32;
                                }
                            }
                            room_size.x += self.delta as f32;
                            room_position.x -= self.delta as f32 * grid_size;
                        } else if self.delta < 0 {
                            let shrink = (-self.delta) as usize;
                            if map.width > shrink {
                                map.width -= shrink;
                                for exit in exits.iter_mut() {
                                    let on_left = (exit.position.x + 1.0).abs() < f32::EPSILON;
                                    let on_right = (exit.position.x - room_size.x).abs() < f32::EPSILON;
                                    if on_left {
                                        exit.position.x += shrink as f32;
                                    } else if on_right {
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
                            map.width += self.delta as usize;
                            for exit in exits.iter_mut() {
                                if (exit.position.x - room_size.x).abs() < f32::EPSILON {
                                    exit.position.x += self.delta as f32;
                                }
                            }
                            room_size.x += self.delta as f32;
                        } else if self.delta < 0 {
                            let shrink = (-self.delta) as usize;
                            if map.width > shrink {
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

                current_world.rebuild_room_grid();
            }

            let ctx = &mut editor.game.ctx_mut();
            replace_room_tile_placements(ctx, self.room_id, &next_placements);
        });
    }

    fn undo(&mut self) {
        with_editor(|editor| {
            {
                let Some(current_world) = editor.game.current_world_mut() else {
                    return;
                };
                let Some(room) = current_world
                    .rooms_mut()
                    .iter_mut()
                    .find(|room| room.id == self.room_id)
                else {
                    return;
                };

                room.position = self.old_position;
                room.size = self.old_size;
                room.exits = self.old_exits.clone();

                let map = &mut room.variants[self.variant_index].tilemap;
                map.width = self.old_width;
                map.height = self.old_height;

                current_world.rebuild_room_grid();
            }

            let ctx = &mut editor.game.ctx_mut();
            replace_room_tile_placements(ctx, self.room_id, &self.old_placements);
        });
    }

    fn applies_in_mode(&self, current_mode: EditorMode) -> bool {
        current_mode == EditorMode::Room(self.room_id)
    }
}

fn room_tile_placements(ecs: &Ecs, room_id: RoomId) -> Vec<TilePlacement> {
    ecs.entities_in_room(room_id)
        .iter()
        .copied()
        .filter_map(|entity| ecs.get::<TilePlacement>(entity).copied())
        .collect()
}

fn replace_room_tile_placements(
    ctx: &mut GameCtxMut<'_>,
    room_id: RoomId,
    placements: &[TilePlacement],
) {
    let existing_entities: Vec<_> = ctx
        .ecs
        .entities_in_room(room_id)
        .iter()
        .copied()
        .filter(|entity| ctx.ecs.get::<TilePlacement>(*entity).is_some())
        .collect();

    for entity in existing_entities {
        Ecs::remove_entity(ctx, entity);
    }

    for &placement in placements {
        let entity = ctx
            .ecs
            .create_entity()
            .with(placement)
            .with_current_room(room_id)
            .finish();
        apply_tile_placement_definition(ctx, entity);
    }
}

fn resized_placements(
    placements: &[TilePlacement],
    side: HandleSide,
    delta: i32,
    old_width: usize,
    old_height: usize,
) -> Vec<TilePlacement> {
    let mut next = placements.to_vec();

    match side {
        HandleSide::Top => {
            if delta > 0 {
                let shift = delta as usize;
                for placement in &mut next {
                    placement.grid_y += shift;
                }
            } else if delta < 0 {
                let shrink = (-delta) as usize;
                if shrink >= old_height {
                    return next;
                }
                next.retain(|placement| placement.grid_y >= shrink);
                for placement in &mut next {
                    placement.grid_y -= shrink;
                }
            }
        }
        HandleSide::Bottom => {
            if delta < 0 {
                let shrink = (-delta) as usize;
                if shrink >= old_height {
                    return next;
                }
                let new_height = old_height - shrink;
                next.retain(|placement| placement.grid_y < new_height);
            }
        }
        HandleSide::Left => {
            if delta > 0 {
                let shift = delta as usize;
                for placement in &mut next {
                    placement.grid_x += shift;
                }
            } else if delta < 0 {
                let shrink = (-delta) as usize;
                if shrink >= old_width {
                    return next;
                }
                next.retain(|placement| placement.grid_x >= shrink);
                for placement in &mut next {
                    placement.grid_x -= shrink;
                }
            }
        }
        HandleSide::Right => {
            if delta < 0 {
                let shrink = (-delta) as usize;
                if shrink >= old_width {
                    return next;
                }
                let new_width = old_width - shrink;
                next.retain(|placement| placement.grid_x < new_width);
            }
        }
    }

    next
}
