mod alt_copy;
mod box_selection;
mod drag_motion;
mod pointer_down;
mod state;

use self::alt_copy::{enter_alt_copy_mode, exit_alt_copy_mode};
use self::box_selection::finish_box_selection;
use self::drag_motion::step_active_entity_drag;
use self::pointer_down::try_begin_pointer_interaction;
use self::state::EntityDragCommand;
pub(crate) use self::state::DragState;
use crate::app::{EditorMode, SubEditor};
use crate::commands::room::*;
use crate::editor_global::*;
use crate::gui::inspector::collider_module::edit::ColliderEditConfig;
use crate::room::collider_drag::{
    apply_collider_edit_nudge,
    collider_update_command,
    step_active_collider_drag,
};
use crate::room::interactable_drag::{
    apply_interactable_edit_nudge,
    interactable_update_command,
    step_active_interactable_drag,
};
use crate::room::room_editor::*;
use crate::room::selection::*;
use crate::shared::input::shortcuts_blocked;
use crate::world::coord;
use bishop::prelude::*;
use engine_core::assets::*;
use engine_core::controls::get_omni_input_pressed;
use engine_core::ecs::*;
use engine_core::worlds::*;
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
            self.active_layer_state.active_layer,
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
        if try_begin_pointer_interaction(self, ctx, room_id, camera, ecs, sprite_manager, grid_size)
        {
            return true;
        }

        if finish_box_selection(self, ctx, camera, room_id, ecs, sprite_manager, grid_size) {
            return true;
        }

        if self.drag_state.entity_drag.dragging {
            let mouse_world = coord::mouse_world_pos(ctx, camera);
            let alt_just_pressed =
                ctx.is_key_pressed(KeyCode::LeftAlt) || ctx.is_key_pressed(KeyCode::RightAlt);
            if !self.drag_state.entity_drag.alt_copy_mode && alt_just_pressed {
                enter_alt_copy_mode(self, ecs, room_id);
            }

            let alt_just_released =
                ctx.is_key_released(KeyCode::LeftAlt) || ctx.is_key_released(KeyCode::RightAlt);
            if self.drag_state.entity_drag.alt_copy_mode && alt_just_released {
                exit_alt_copy_mode(self, ecs, mouse_world);
            }

            let mouse_released = ctx.is_mouse_button_released(MouseButton::Left);
            let should_commit_move = mouse_released && !self.drag_state.entity_drag.alt_copy_mode;
            if let Some(command) = step_active_entity_drag(
                &mut self.drag_state.entity_drag,
                ecs,
                mouse_world,
                should_commit_move,
                ctx.is_key_down(KeyCode::S),
                grid_size,
            ) {
                push_entity_drag_command(room_id, command);
            }

            if mouse_released && self.drag_state.entity_drag.alt_copy_mode {
                if !self.drag_state.entity_drag.alt_copied_entities.is_empty() {
                    let copied = std::mem::take(&mut self.drag_state.entity_drag.alt_copied_entities);
                    push_entity_drag_command(
                        room_id,
                        EntityDragCommand::AltCopy {
                            copied_entities: copied,
                        },
                    );
                }
                self.drag_state.entity_drag.clear();
            }
            return true;
        }

        let snap_held = ctx.is_key_down(KeyCode::S);
        let shift_held =
            ctx.is_key_down(KeyCode::LeftShift) || ctx.is_key_down(KeyCode::RightShift);
        let config = ColliderEditConfig {
            grid_size,
            snap_enabled: snap_held,
            shift_held,
        };

        let interactable_result = step_active_interactable_drag(
            &mut self.drag_state.interactable_drag,
            ecs,
            coord::mouse_world_pos(ctx, camera),
            ctx.is_mouse_button_down(MouseButton::Left),
            ctx.is_mouse_button_released(MouseButton::Left),
            config,
        );
        if interactable_result.consumed {
            if let Some((entity, old_interactable, new_interactable)) = interactable_result.commit {
                push_command(interactable_update_command(
                    entity,
                    old_interactable,
                    new_interactable,
                    EditorMode::Room(room_id),
                ));
            }
            return true;
        }

        let result = step_active_collider_drag(
            &mut self.drag_state.collider_drag,
            ecs,
            coord::mouse_world_pos(ctx, camera),
            ctx.is_mouse_button_down(MouseButton::Left),
            ctx.is_mouse_button_released(MouseButton::Left),
            config,
        );
        if result.consumed {
            if let Some((entity, old_collider, new_collider)) = result.commit {
                push_command(collider_update_command(
                    entity,
                    old_collider,
                    new_collider,
                    EditorMode::Room(room_id),
                ));
            }
            return true;
        }

        false
    }

    /// Nudges the selected collider offset in edit mode, otherwise moves selected entities.
    pub(crate) fn handle_keyboard_move(
        &mut self,
        ctx: &WgpuContext,
        ecs: &mut Ecs,
        room_id: RoomId,
    ) {
        if self.drag_state.entity_drag.dragging
            || self.selected_entities.is_empty()
            || shortcuts_blocked()
        {
            return;
        }

        let dir = get_omni_input_pressed(ctx);
        if dir.length_squared() == 0.0 {
            return;
        }

        let step = dir;
        if let Some(cmd) = apply_interactable_edit_nudge(
            self.single_selected_entity(),
            ecs,
            room_id,
            self.active_layer_state.active_layer,
            step,
        ) {
            push_command(cmd);
            return;
        }

        if let Some(cmd) = apply_collider_edit_nudge(
            self.single_selected_entity(),
            ecs,
            room_id,
            self.active_layer_state.active_layer,
            step,
        ) {
            push_command(cmd);
            return;
        }

        let mut moves = Vec::new();
        let active_layer = self.active_layer_state.active_layer;

        for &entity in &self.selected_entities {
            if !can_select_entity_in_room_layer(ecs, entity, room_id, active_layer) {
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
        let mut all_snapshots = Vec::new();

        for &entity in &self.selected_entities {
            if ecs.has::<Player>(entity) {
                continue;
            }
            all_snapshots.extend(capture_subtree(ecs, entity));
        }

        if all_snapshots.is_empty() {
            return Vec::new();
        }

        let snapshot_ids: HashSet<Entity> = all_snapshots.iter().map(|snapshot| snapshot.entity).collect();
        let mut root_old_ids = Vec::new();

        for snapshot in &all_snapshots {
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

        let mut id_map = HashMap::new();
        for snapshot in &all_snapshots {
            let new_id = ecs.create_entity().finish();
            id_map.insert(snapshot.entity, new_id);
        }

        for snapshot in &all_snapshots {
            let new_id = id_map[&snapshot.entity];

            for comp in &snapshot.components {
                let Some((reg, mut boxed)) = restore_component_with_remap(comp, &id_map) else {
                    continue;
                };

                if comp.type_name == comp_type_name::<Animation>() {
                    if let Some(anim) = boxed.as_mut().downcast_mut::<Animation>() {
                        anim.init_runtime();
                    }
                }

                ecs.insert_component_dyn(reg, new_id, boxed);
            }
        }

        root_old_ids
            .into_iter()
            .filter_map(|old| id_map.get(&old).map(|&new| (old, new)))
            .collect()
    }
}

fn push_entity_drag_command(room_id: RoomId, command: EntityDragCommand) {
    match command {
        EntityDragCommand::MoveOne { entity, from, to } => {
            push_command(Box::new(MoveEntityCmd::new(
                entity,
                EditorMode::Room(room_id),
                from,
                to,
            )));
        }
        EntityDragCommand::MoveMany { moves } => {
            push_command(Box::new(BatchMoveEntitiesCmd::new(
                moves,
                EditorMode::Room(room_id),
            )));
        }
        EntityDragCommand::AltCopy { copied_entities } => {
            push_command(Box::new(AltDragCopyCmd::new(
                copied_entities,
                EditorMode::Room(room_id),
            )));
        }
    }
}

#[cfg(test)]
mod tests;
