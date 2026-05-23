// editor/src/room/entity_drag.rs
use crate::app::EditorMode;
use crate::app::SubEditor;
use crate::commands::room::*;
use crate::editor_global::*;
use crate::room::room_editor::*;
use crate::room::selection::*;
use crate::shared::input::shortcuts_blocked;
use crate::shared::selection::*;
use crate::world::coord;
use bishop::prelude::*;
use engine_core::prelude::*;
use std::collections::{HashMap, HashSet};

impl RoomEditor {
    pub(crate) fn handle_prefab_stamp(
        &mut self,
        ctx: &WgpuContext,
        camera: &Camera2D,
        room_id: RoomId,
        grid_size: f32,
        active_prefab_stamp: ActivePrefabStampState,
    ) -> bool {
        let Some(prefab_id) = self.active_prefab_id else {
            return false;
        };
        if self.mode != RoomEditorMode::Scene
            || self.scene_sub_mode != RoomSceneSubMode::Stamp
            || self.should_block_canvas(ctx)
            || !ctx.is_mouse_button_pressed(MouseButton::Left)
        {
            return false;
        }
        if !active_prefab_stamp.available {
            self.active_prefab_id = None;
            self.reset_scene_sub_mode();
            return false;
        }

        let mouse_world = coord::mouse_world_pos(ctx, camera);
        let snapped_position =
            snap_room_drag_position(mouse_world, grid_size, active_prefab_stamp.pivot);
        push_command(Box::new(PlacePrefabInstanceCmd::new(
            prefab_id,
            room_id,
            snapped_position,
            EditorMode::Room(room_id),
        )));
        true
    }

    /// Handles mouse selection / movement with multi-select support.
    pub(crate) fn handle_selection(
        &mut self,
        ctx: &mut WgpuContext,
        room_id: RoomId,
        camera: &Camera2D,
        ecs: &mut Ecs,
        sprite_manager: &mut SpriteManager,
        grid_size: f32,
    ) -> bool {
        let mouse_screen: Vec2 = ctx.mouse_position().into();
        let ui_was_clicked = self.ui_was_clicked(ctx);
        let shift_held =
            ctx.is_key_down(KeyCode::LeftShift) || ctx.is_key_down(KeyCode::RightShift);
        let mouse_world = coord::mouse_world_pos(ctx, camera);

        // Handle mouse button press
        if !ui_was_clicked
            && ctx.is_mouse_button_pressed(MouseButton::Left)
            && !self.drag_state.dragging
            && !self.drag_state.box_select_active
        {
            // Find ALL entities under cursor and select topmost by z-order
            // Tuple: (entity, z, is_camera) - cameras always on top
            let mut candidates: Vec<(Entity, i32, bool)> = Vec::new();
            let layer_store = ecs.get_store::<Layer>();
            let camera_store = ecs.get_store::<RoomCamera>();

            for (entity, pos) in ecs.get_store::<Transform>().data.iter() {
                if !can_select_entity_in_room(ecs, *entity, room_id) {
                    continue;
                }
                let hitbox = entity_hitbox(
                    ctx,
                    *entity,
                    pos.position,
                    camera,
                    ecs,
                    sprite_manager,
                    grid_size,
                );
                if hitbox.contains(mouse_screen) {
                    let z = layer_store.get(*entity).map_or(0, |l| l.z);
                    let is_camera = camera_store.get(*entity).is_some();
                    candidates.push((*entity, z, is_camera));
                }
            }

            // Sort: cameras first, then by z descending (highest z = visually on top)
            candidates.sort_by(|a, b| match (a.2, b.2) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => b.1.cmp(&a.1),
            });
            let clicked_entity = candidates.first().map(|(e, _, _)| *e);

            if let Some(entity) = clicked_entity {
                // Clicked on an entity
                if shift_held {
                    // Toggle entity in selection
                    if self.selected_entities.contains(&entity) {
                        self.selected_entities.remove(&entity);
                    } else {
                        self.selected_entities.insert(entity);
                    }
                } else {
                    // Clear and select single, start drag
                    if !self.selected_entities.contains(&entity) {
                        self.selected_entities.clear();
                        self.selected_entities.insert(entity);
                    }

                    // Start normal drag
                    self.drag_state.dragging = true;
                    self.drag_state.drag_anchor_entity = Some(entity);
                    self.drag_state.drag_offset = ecs
                        .get_store::<Transform>()
                        .get(entity)
                        .map(|t| t.position - mouse_world)
                        .unwrap_or(Vec2::ZERO);

                    // Store start positions for all selected entities
                    self.drag_state.drag_start_positions.clear();
                    for &e in &self.selected_entities {
                        if let Some(pos) = ecs.get_store::<Transform>().get(e).map(|t| t.position) {
                            self.drag_state.drag_start_positions.push((e, pos));
                        }
                    }

                    // Store initial positions for undo command
                    self.drag_state.drag_initial_start_positions =
                        self.drag_state.drag_start_positions.clone();

                    // If alt is already held, immediately enter copy mode
                    let alt_held =
                        ctx.is_key_down(KeyCode::LeftAlt) || ctx.is_key_down(KeyCode::RightAlt);
                    if alt_held {
                        // Store original drag state for reverting on alt release
                        self.drag_state.pre_copy_drag_state = Some(PreCopyDragState {
                            anchor_entity: self.drag_state.drag_anchor_entity,
                            selected_entities: self.selected_entities.clone(),
                        });

                        // Create duplicates at current positions
                        let duplicates = self.duplicate_entities_for_drag(ecs, room_id);
                        if !duplicates.is_empty() {
                            // Position duplicates where originals are
                            for (orig, dup) in &duplicates {
                                if let Some((_, pos)) = self
                                    .drag_state
                                    .drag_start_positions
                                    .iter()
                                    .find(|(e, _)| e == orig)
                                {
                                    update_entity_position(ecs, *dup, *pos);
                                }
                            }

                            // Find the duplicate corresponding to the anchor
                            let new_anchor = self
                                .drag_state
                                .drag_anchor_entity
                                .and_then(|anchor| duplicates.iter().find(|(o, _)| *o == anchor))
                                .map(|(_, d)| *d)
                                .unwrap_or(duplicates[0].1);

                            // Update selection to duplicates
                            self.selected_entities.clear();
                            for (_, dup) in &duplicates {
                                self.selected_entities.insert(*dup);
                            }

                            // Update drag tracking to use duplicates
                            self.drag_state.drag_start_positions = duplicates
                                .iter()
                                .filter_map(|(orig, dup)| {
                                    self.drag_state
                                        .drag_initial_start_positions
                                        .iter()
                                        .find(|(e, _)| e == orig)
                                        .map(|(_, pos)| (*dup, *pos))
                                })
                                .collect();

                            self.drag_state.drag_anchor_entity = Some(new_anchor);
                            self.drag_state.alt_copied_entities =
                                duplicates.iter().map(|(_, d)| *d).collect();
                            self.drag_state.alt_copy_mode = true;
                        }
                    }
                }
            } else {
                // Clicked on empty space
                if shift_held {
                    // Start box selection
                    self.drag_state.box_select_start = Some(mouse_world);
                    self.drag_state.box_select_active = true;
                } else {
                    // Clear selection and start box selection
                    self.selected_entities.clear();
                    self.drag_state.box_select_start = Some(mouse_world);
                    self.drag_state.box_select_active = true;
                }
            }
        }

        // Handle box selection
        if self.drag_state.box_select_active {
            if ctx.is_mouse_button_released(MouseButton::Left) {
                // Finish box selection
                if let Some(start) = self.drag_state.box_select_start.take() {
                    let box_rect = rect_from_two_points(start, mouse_world);

                    // Find all entities within the box
                    for (entity, pos) in ecs.get_store::<Transform>().data.iter() {
                        if !can_select_entity_in_room(ecs, *entity, room_id) {
                            continue;
                        }
                        let entity_rect = entity_world_rect(
                            *entity,
                            pos.position,
                            ecs,
                            sprite_manager,
                            grid_size,
                        );
                        if rects_intersect(box_rect, entity_rect) {
                            self.selected_entities.insert(*entity);
                        }
                    }
                }
                self.drag_state.box_select_active = false;
            }
            return true;
        }

        // Execute the drag while the button is held
        if self.drag_state.dragging {
            // Check if alt was just pressed mid-drag to switch to copy mode
            let alt_just_pressed =
                ctx.is_key_pressed(KeyCode::LeftAlt) || ctx.is_key_pressed(KeyCode::RightAlt);
            if !self.drag_state.alt_copy_mode && alt_just_pressed {
                // Get current positions of originals
                let current_positions: Vec<(Entity, Vec2)> = self
                    .drag_state
                    .drag_start_positions
                    .iter()
                    .filter_map(|(e, _)| {
                        ecs.get_store::<Transform>()
                            .get(*e)
                            .map(|t| (*e, t.position))
                    })
                    .collect();

                // Store original drag state for reverting on alt release
                self.drag_state.pre_copy_drag_state = Some(PreCopyDragState {
                    anchor_entity: self.drag_state.drag_anchor_entity,
                    selected_entities: self.selected_entities.clone(),
                });

                // Create duplicates at current positions
                let duplicates = self.duplicate_entities_for_drag(ecs, room_id);
                if !duplicates.is_empty() {
                    // Position duplicates where originals currently are
                    for (orig, dup) in &duplicates {
                        if let Some((_, pos)) = current_positions.iter().find(|(e, _)| e == orig) {
                            update_entity_position(ecs, *dup, *pos);
                        }
                    }

                    // Move originals back to their initial start positions
                    for (entity, initial_pos) in &self.drag_state.drag_initial_start_positions {
                        update_entity_position(ecs, *entity, *initial_pos);
                    }

                    // Find the duplicate corresponding to the anchor
                    let new_anchor = self
                        .drag_state
                        .drag_anchor_entity
                        .and_then(|anchor| duplicates.iter().find(|(o, _)| *o == anchor))
                        .map(|(_, d)| *d)
                        .unwrap_or(duplicates[0].1);

                    // Update selection to duplicates
                    self.selected_entities.clear();
                    for (_, dup) in &duplicates {
                        self.selected_entities.insert(*dup);
                    }

                    // Update drag tracking to use duplicates
                    self.drag_state.drag_start_positions = duplicates
                        .iter()
                        .filter_map(|(orig, dup)| {
                            current_positions
                                .iter()
                                .find(|(e, _)| e == orig)
                                .map(|(_, pos)| (*dup, *pos))
                        })
                        .collect();

                    self.drag_state.drag_anchor_entity = Some(new_anchor);
                    self.drag_state.alt_copied_entities =
                        duplicates.iter().map(|(_, d)| *d).collect();
                    self.drag_state.alt_copy_mode = true;
                }
            }

            // Check if alt was just released mid-drag to revert copy mode
            let alt_just_released =
                ctx.is_key_released(KeyCode::LeftAlt) || ctx.is_key_released(KeyCode::RightAlt);
            if self.drag_state.alt_copy_mode && alt_just_released {
                if let Some(original_state) = self.drag_state.pre_copy_drag_state.take() {
                    // Get current positions of copies before deleting them
                    let copy_positions: Vec<(Entity, Vec2)> = self
                        .drag_state
                        .alt_copied_entities
                        .iter()
                        .filter_map(|e| {
                            ecs.get_store::<Transform>()
                                .get(*e)
                                .map(|t| (*e, t.position))
                        })
                        .collect();

                    // Build mapping from copy to original
                    let copy_to_orig: Vec<(Entity, Entity)> = self
                        .drag_state
                        .alt_copied_entities
                        .iter()
                        .zip(original_state.selected_entities.iter())
                        .map(|(c, o)| (*c, *o))
                        .collect();

                    // Delete the copied entities
                    for &copy_entity in &self.drag_state.alt_copied_entities {
                        ecs.remove_entity_components(copy_entity);
                    }
                    self.drag_state.alt_copied_entities.clear();

                    // Restore original selection and anchor
                    self.selected_entities = original_state.selected_entities;
                    self.drag_state.drag_anchor_entity = original_state.anchor_entity;

                    // Move originals to where copies were (under the mouse)
                    self.drag_state.drag_start_positions.clear();
                    for (copy, orig) in &copy_to_orig {
                        if let Some((_, copy_pos)) = copy_positions.iter().find(|(e, _)| e == copy)
                        {
                            update_entity_position(ecs, *orig, *copy_pos);
                            self.drag_state
                                .drag_start_positions
                                .push((*orig, *copy_pos));
                        }
                    }

                    // Update drag_offset so drag continues smoothly from current position
                    if let Some(anchor) = self.drag_state.drag_anchor_entity {
                        self.drag_state.drag_offset = ecs
                            .get_store::<Transform>()
                            .get(anchor)
                            .map(|t| t.position - mouse_world)
                            .unwrap_or(Vec2::ZERO);
                    }

                    self.drag_state.alt_copy_mode = false;
                }
            }

            // Find the anchor entity's start position and move entities
            let anchor_start = self.drag_state.drag_anchor_entity.and_then(|anchor| {
                self.drag_state
                    .drag_start_positions
                    .iter()
                    .find(|(e, _)| *e == anchor)
                    .map(|(_, pos)| *pos)
            });

            if let Some(anchor_start) = anchor_start {
                let anchor_entity = self.drag_state.drag_anchor_entity.unwrap();
                let target_pos = mouse_world + self.drag_state.drag_offset;

                // Optionally snap to grid (based on anchor entity)
                let final_target = if ctx.is_key_down(KeyCode::S) {
                    let pivot = ecs
                        .get_store::<Transform>()
                        .get(anchor_entity)
                        .map(|t| t.pivot)
                        .unwrap_or(Pivot::BottomCenter);
                    snap_room_drag_position(mouse_world, grid_size, pivot)
                } else {
                    target_pos
                };

                // Move all selected entities by the same delta
                let delta = final_target - anchor_start;
                for &(entity, start_pos) in &self.drag_state.drag_start_positions {
                    update_entity_position(ecs, entity, start_pos + delta);
                }
            }

            // Finish the drag when the button is released
            if ctx.is_mouse_button_released(MouseButton::Left) {
                if self.drag_state.alt_copy_mode {
                    // Alt+drag copy: push command for the duplicated entities
                    if !self.drag_state.alt_copied_entities.is_empty() {
                        let copied = std::mem::take(&mut self.drag_state.alt_copied_entities);
                        push_command(Box::new(AltDragCopyCmd::new(
                            copied,
                            EditorMode::Room(room_id),
                        )));
                    }
                    self.drag_state.alt_copy_mode = false;
                } else {
                    // Normal drag: build moves list for undo command
                    let mut moves = Vec::new();
                    for &(entity, initial_pos) in &self.drag_state.drag_initial_start_positions {
                        if let Some(final_pos) =
                            ecs.get_store::<Transform>().get(entity).map(|t| t.position)
                        {
                            if (final_pos - initial_pos).length_squared() > 0.0 {
                                moves.push((entity, initial_pos, final_pos));
                            }
                        }
                    }

                    // Push command only if something moved
                    if !moves.is_empty() {
                        if moves.len() == 1 {
                            let (entity, from, to) = moves[0];
                            push_command(Box::new(MoveEntityCmd::new(
                                entity,
                                EditorMode::Room(room_id),
                                from,
                                to,
                            )));
                        } else {
                            push_command(Box::new(BatchMoveEntitiesCmd::new(
                                moves,
                                EditorMode::Room(room_id),
                            )));
                        }
                    }
                }

                self.drag_state.drag_start_positions.clear();
                self.drag_state.drag_initial_start_positions.clear();
                self.drag_state.drag_anchor_entity = None;
                self.drag_state.dragging = false;
                self.drag_state.pre_copy_drag_state = None;
            }
            return true;
        }
        false
    }

    /// Moves all selected entities by one pixel using arrow keys.
    pub(crate) fn handle_keyboard_move(
        &mut self,
        ctx: &WgpuContext,
        ecs: &mut Ecs,
        room_id: RoomId,
    ) {
        if self.drag_state.dragging || self.selected_entities.is_empty() || shortcuts_blocked() {
            return;
        }

        let dir = get_omni_input_pressed(ctx);
        if dir.length_squared() == 0.0 {
            return;
        }

        let step = dir;
        let mut moves = Vec::new();

        for &entity in &self.selected_entities {
            if !can_select_entity_in_room(ecs, entity, room_id) {
                continue;
            }

            if let Some(transform) = ecs.get_store_mut::<Transform>().get_mut(entity) {
                let old = transform.position;
                transform.position += step;
                moves.push((entity, old, transform.position));
            }
        }

        if !moves.is_empty() {
            if moves.len() == 1 {
                let (entity, from, to) = moves[0];
                push_command(Box::new(MoveEntityCmd::new(
                    entity,
                    EditorMode::Room(room_id),
                    from,
                    to,
                )));
            } else {
                push_command(Box::new(BatchMoveEntitiesCmd::new(
                    moves,
                    EditorMode::Room(room_id),
                )));
            }
        }
    }

    /// Duplicates selected entities for alt+drag copy operation.
    /// Returns a vec of (original_entity, duplicate_entity) pairs.
    pub(crate) fn duplicate_entities_for_drag(
        &self,
        ecs: &mut Ecs,
        _room_id: RoomId,
    ) -> Vec<(Entity, Entity)> {
        // Capture snapshots of all selected entities
        let mut all_snapshots = Vec::new();
        let mut entity_order = Vec::new();

        for &entity in &self.selected_entities {
            if ecs.has::<Player>(entity) {
                continue;
            }
            let group_snapshot = capture_subtree(ecs, entity);
            for snapshot in &group_snapshot {
                entity_order.push(snapshot.entity);
            }
            all_snapshots.extend(group_snapshot);
        }

        if all_snapshots.is_empty() {
            return Vec::new();
        }

        // Find root entities (entities without parents in the snapshot)
        let snapshot_ids: HashSet<Entity> = all_snapshots.iter().map(|s| s.entity).collect();
        let mut root_old_ids = Vec::new();

        for snapshot in all_snapshots.iter() {
            let has_parent_in_snapshot = snapshot.components.iter().any(|comp| {
                if comp.type_name == comp_type_name::<Parent>() {
                    if let Ok(parent) = ron::from_str::<Parent>(&comp.ron) {
                        return snapshot_ids.contains(&parent.0);
                    }
                }
                false
            });

            if !has_parent_in_snapshot {
                root_old_ids.push(snapshot.entity);
            }
        }

        // Create new entities for each snapshot entry
        let mut id_map = HashMap::new();
        for snapshot in all_snapshots.iter() {
            let new_id = ecs.create_entity().finish();
            id_map.insert(snapshot.entity, new_id);
        }

        // Restore components to the new entities
        for snapshot in all_snapshots.iter() {
            let new_id = id_map[&snapshot.entity];

            for comp in snapshot.components.iter() {
                let Some((reg, mut boxed)) = restore_component_with_remap(comp, &id_map) else {
                    continue;
                };

                // Initialize Animation runtime state so it renders during drag
                if comp.type_name == comp_type_name::<Animation>() {
                    if let Some(anim) = boxed.as_mut().downcast_mut::<Animation>() {
                        anim.init_runtime();
                    }
                }

                ecs.insert_component_dyn(reg, new_id, boxed);
            }
        }

        // Return mapping of root entities only (original -> duplicate)
        root_old_ids
            .into_iter()
            .filter_map(|old| id_map.get(&old).map(|&new| (old, new)))
            .collect()
    }
}
