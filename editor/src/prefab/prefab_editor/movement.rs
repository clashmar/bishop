use super::selection::is_prefab_entity;
use super::{PrefabDragState, PrefabEditor, PREFAB_EDITOR_GRID_SIZE};
use crate::app::EditorMode;
use crate::commands::room::{BatchMoveEntitiesCmd, MoveEntityCmd};
use crate::commands::scene::UpdateComponentCmd;
use crate::editor_global::push_command;
use crate::gui::inspector::collider_module::edit::{ColliderEditConfig, is_collider_edit_active_for};
use crate::gui::inspector::interactable_module::edit::is_interactable_edit_active_for;
use crate::room::collider_drag::{
    ColliderDragStep,
    collider_update_command,
    step_active_collider_drag,
    try_intercept_collider_handle,
    try_start_collider_handle_on_click,
};
use crate::room::entity_hitbox;
use crate::room::entity_world_rect;
use crate::room::interactable_drag::{
    InteractableDragStep,
    interactable_update_command,
    step_active_interactable_drag,
    try_intercept_interactable_handle,
    try_start_interactable_handle_on_click,
};
use crate::shared::input::shortcuts_blocked;
use crate::shared::selection::{rect_from_two_points, rects_intersect};
use crate::world::coord;
use bishop::prelude::*;
use engine_core::assets::*;
use engine_core::controls::{get_omni_input_pressed};
use engine_core::ecs::*;
use engine_core::rendering::resolve_visual_entity;

impl PrefabEditor {
    pub(crate) fn handle_canvas_move(
        &mut self,
        ctx: &WgpuContext,
        camera: &Camera2D,
        ecs: &mut Ecs,
        sprite_manager: &mut SpriteManager,
    ) -> bool {
        let shift_held =
            ctx.is_key_down(KeyCode::LeftShift) || ctx.is_key_down(KeyCode::RightShift);
        let mouse_screen: Vec2 = ctx.mouse_position().into();
        let mouse_world = coord::mouse_world_pos(ctx, camera);

        if self.step_active_bounds_drag(ctx, ecs, mouse_world) {
            return true;
        }

        if self.drag_state.dragging {
            let anchor_start = self.drag_state.drag_anchor_entity.and_then(|anchor| {
                self.drag_state
                    .drag_start_positions
                    .iter()
                    .find(|(entity, _)| *entity == anchor)
                    .map(|(_, pos)| *pos)
            });

            if let Some(anchor_start) = anchor_start {
                let delta = mouse_world + self.drag_state.drag_offset - anchor_start;
                for &(entity, start_pos) in &self.drag_state.drag_start_positions {
                    update_entity_position(ecs, entity, start_pos + delta);
                }
            }

            if ctx.is_mouse_button_released(MouseButton::Left) {
                let initial_positions =
                    std::mem::take(&mut self.drag_state.drag_initial_start_positions);
                self.finish_move_command(ecs, &initial_positions);
                self.drag_state = PrefabDragState::default();
            }

            return true;
        }

        if self.drag_state.box_select_active {
            if ctx.is_mouse_button_released(MouseButton::Left) {
                if let Some(start) = self.drag_state.box_select_start.take() {
                    let box_rect = rect_from_two_points(start, mouse_world);
                    let selection_len = self.selected_entities.len();
                    for (entity, transform) in ecs.get_store::<Transform>().data.iter() {
                        if !is_prefab_entity(ecs, *entity) {
                            continue;
                        }
                        let entity_rect = entity_world_rect(
                            *entity,
                            transform.position,
                            ecs,
                            sprite_manager,
                            PREFAB_EDITOR_GRID_SIZE,
                        );
                        if rects_intersect(box_rect, entity_rect) {
                            self.selected_entities.insert(*entity);
                        }
                    }
                    if self.selected_entities.len() != selection_len {
                        self.disable_active_edit_modes();
                    }
                }
                self.drag_state.box_select_active = false;
                self.sync_inspector_to_selection();
            }
            return true;
        }

        if !ctx.is_mouse_button_pressed(MouseButton::Left) {
            return false;
        }

        if self.try_begin_active_bounds_drag(ecs, mouse_world) {
            return true;
        }

        let mut candidates = Vec::new();
        for (entity, transform) in ecs.get_store::<Transform>().data.iter() {
            if !is_prefab_entity(ecs, *entity) {
                continue;
            }

            let hitbox = entity_hitbox(
                ctx,
                *entity,
                transform.position,
                camera,
                ecs,
                sprite_manager,
                PREFAB_EDITOR_GRID_SIZE,
            );

            if hitbox.contains(mouse_screen) {
                let z = ecs
                    .get_store::<Layer>()
                    .get(*entity)
                    .map_or(0, |layer| layer.z);
                candidates.push((*entity, z));
            }
        }

        candidates.sort_by(|a, b| b.1.cmp(&a.1));
        let clicked_entity = candidates.first().map(|(entity, _)| *entity);

        match (shift_held, clicked_entity) {
            (true, Some(entity)) => self.toggle_entity_selection(entity),
            (false, Some(entity)) => {
                if !self.selected_entities.contains(&entity) {
                    self.set_selected_entity(Some(entity));
                }
                if self.try_begin_bounds_drag_on_clicked_entity(ecs, entity, mouse_world) {
                    return true;
                }
                self.start_drag(ecs, entity, mouse_world);
            }
            (false, None) => {
                self.clear_selection();
                self.drag_state.box_select_start = Some(mouse_world);
                self.drag_state.box_select_active = true;
            }
            (true, None) => {
                self.drag_state.box_select_start = Some(mouse_world);
                self.drag_state.box_select_active = true;
            }
        }

        self.drag_state.dragging || self.drag_state.box_select_active
    }

    pub(crate) fn handle_keyboard_move(&mut self, ctx: &WgpuContext, ecs: &mut Ecs) {
        if self.drag_state.dragging || self.selected_entities.is_empty() || shortcuts_blocked() {
            return;
        }

        let delta = get_omni_input_pressed(ctx);
        if delta.length_squared() == 0.0 {
            return;
        }

        if let Some(cmd) = self.try_nudge_active_bounds(ecs, delta) {
            push_command(cmd);
            return;
        }

        self.move_selected_entities_by(ecs, delta);
    }

    pub(crate) fn move_selected_entities_by(&mut self, ecs: &mut Ecs, delta: Vec2) {
        if delta.length_squared() == 0.0 {
            return;
        }

        let mut moves = Vec::new();
        for entity in self.movable_selected_entities() {
            if let Some(transform) = ecs.get_store_mut::<Transform>().get_mut(entity) {
                let old = transform.position;
                transform.position += delta;
                moves.push((entity, old, transform.position));
            }
        }

        self.push_move_command(moves);
    }

    /// Begins an active bounds-handle drag when the mouse hits a prefab edit handle.
    pub(crate) fn try_begin_active_bounds_drag(&mut self, ecs: &Ecs, mouse_world: Vec2) -> bool {
        if let Some((entity, action, interactable)) = try_intercept_interactable_handle(
            self.single_selected_entity(),
            ecs,
            mouse_world,
            PREFAB_EDITOR_GRID_SIZE,
        ) {
            let transform_entity = self.single_selected_entity().unwrap_or(entity);
            self.drag_state.interactable_drag.begin(
                entity,
                transform_entity,
                action,
                interactable,
                mouse_world,
            );
            return true;
        }

        if let Some((entity, action, collider)) = try_intercept_collider_handle(
            self.single_selected_entity(),
            ecs,
            mouse_world,
            PREFAB_EDITOR_GRID_SIZE,
        ) {
            let transform_entity = self.single_selected_entity().unwrap_or(entity);
            self.drag_state.collider_drag.begin(
                entity,
                transform_entity,
                action,
                collider,
                mouse_world,
            );
            return true;
        }

        false
    }

    fn step_active_bounds_drag(
        &mut self,
        ctx: &WgpuContext,
        ecs: &mut Ecs,
        mouse_world: Vec2,
    ) -> bool {
        let config = ColliderEditConfig {
            grid_size: PREFAB_EDITOR_GRID_SIZE,
            snap_enabled: ctx.is_key_down(KeyCode::S),
            shift_held: ctx.is_key_down(KeyCode::LeftShift)
                || ctx.is_key_down(KeyCode::RightShift),
        };

        let interactable_result = step_active_interactable_drag(
            &mut self.drag_state.interactable_drag,
            ecs,
            mouse_world,
            ctx.is_mouse_button_down(MouseButton::Left),
            ctx.is_mouse_button_released(MouseButton::Left),
            config,
        );
        if self.finish_interactable_drag(interactable_result) {
            return true;
        }

        let collider_result = step_active_collider_drag(
            &mut self.drag_state.collider_drag,
            ecs,
            mouse_world,
            ctx.is_mouse_button_down(MouseButton::Left),
            ctx.is_mouse_button_released(MouseButton::Left),
            config,
        );
        self.finish_collider_drag(collider_result)
    }

    fn finish_interactable_drag(&self, result: InteractableDragStep) -> bool {
        if !result.consumed {
            return false;
        }

        if let Some((entity, old_interactable, new_interactable)) = result.commit {
            push_command(interactable_update_command(
                entity,
                old_interactable,
                new_interactable,
                EditorMode::Prefab(self.prefab_id),
            ));
        }
        true
    }

    fn finish_collider_drag(&self, result: ColliderDragStep) -> bool {
        if !result.consumed {
            return false;
        }

        if let Some((entity, old_collider, new_collider)) = result.commit {
            push_command(collider_update_command(
                entity,
                old_collider,
                new_collider,
                EditorMode::Prefab(self.prefab_id),
            ));
        }
        true
    }

    fn try_begin_bounds_drag_on_clicked_entity(
        &mut self,
        ecs: &Ecs,
        entity: Entity,
        mouse_world: Vec2,
    ) -> bool {
        if self.single_selected_entity().is_some_and(is_interactable_edit_active_for) {
            if let Some((interactable_entity, action, interactable)) =
                try_start_interactable_handle_on_click(
                    entity,
                    ecs,
                    mouse_world,
                    PREFAB_EDITOR_GRID_SIZE,
                )
            {
                self.drag_state.interactable_drag.begin(
                    interactable_entity,
                    entity,
                    action,
                    interactable,
                    mouse_world,
                );
                return true;
            }
        }

        if self
            .single_selected_entity()
            .is_some_and(|selected| is_collider_edit_active_for(resolve_visual_entity(ecs, selected)))
        {
            if let Some((collider_entity, action, collider)) = try_start_collider_handle_on_click(
                entity,
                ecs,
                mouse_world,
                PREFAB_EDITOR_GRID_SIZE,
            ) {
                self.drag_state.collider_drag.begin(
                    collider_entity,
                    entity,
                    action,
                    collider,
                    mouse_world,
                );
                return true;
            }
        }

        false
    }

    fn try_nudge_active_bounds(
        &self,
        ecs: &mut Ecs,
        delta: Vec2,
    ) -> Option<Box<UpdateComponentCmd>> {
        let entity = self.single_selected_entity()?;

        if is_interactable_edit_active_for(entity) {
            let old_interactable = ecs.get::<Interactable>(entity)?.clone();
            let mut new_interactable = old_interactable.clone();
            new_interactable.offset += delta;
            if let Some(interactable) = ecs.get_store_mut::<Interactable>().get_mut(entity) {
                *interactable = new_interactable.clone();
            }
            return Some(interactable_update_command(
                entity,
                old_interactable,
                new_interactable,
                EditorMode::Prefab(self.prefab_id),
            ));
        }

        let visual_entity = resolve_visual_entity(ecs, entity);
        if is_collider_edit_active_for(visual_entity) {
            let old_collider = *ecs.get::<Collider>(visual_entity)?;
            let mut new_collider = old_collider;
            new_collider.offset += delta;
            if let Some(collider) = ecs.get_store_mut::<Collider>().get_mut(visual_entity) {
                *collider = new_collider;
            }
            return Some(collider_update_command(
                visual_entity,
                old_collider,
                new_collider,
                EditorMode::Prefab(self.prefab_id),
            ));
        }

        None
    }

    fn movable_selected_entities(&self) -> Vec<Entity> {
        self.selected_entities
            .iter()
            .copied()
            .filter(|entity| Some(*entity) != self.root_entity)
            .collect()
    }

    fn start_drag(&mut self, ecs: &Ecs, entity: Entity, mouse_world: Vec2) {
        if Some(entity) == self.root_entity {
            return;
        }

        let Some(transform) = ecs.get_store::<Transform>().get(entity) else {
            return;
        };

        let drag_start_positions = self
            .movable_selected_entities()
            .into_iter()
            .filter_map(|selected| {
                ecs.get_store::<Transform>()
                    .get(selected)
                    .map(|value| (selected, value.position))
            })
            .collect::<Vec<_>>();
        if drag_start_positions.is_empty() {
            return;
        }

        self.drag_state.dragging = true;
        self.drag_state.drag_anchor_entity = Some(entity);
        self.drag_state.drag_offset = transform.position - mouse_world;
        self.drag_state.drag_start_positions = drag_start_positions.clone();
        self.drag_state.drag_initial_start_positions = drag_start_positions;
    }

    fn finish_move_command(&self, ecs: &Ecs, initial_positions: &[(Entity, Vec2)]) {
        let mut moves = Vec::new();
        for &(entity, initial_pos) in initial_positions {
            if let Some(final_pos) = ecs.get_store::<Transform>().get(entity).map(|t| t.position) {
                if (final_pos - initial_pos).length_squared() > 0.0 {
                    moves.push((entity, initial_pos, final_pos));
                }
            }
        }

        self.push_move_command(moves);
    }

    fn push_move_command(&self, moves: Vec<(Entity, Vec2, Vec2)>) {
        if moves.is_empty() {
            return;
        }

        if moves.len() == 1 {
            let (entity, from, to) = moves[0];
            push_command(Box::new(MoveEntityCmd::new(
                entity,
                EditorMode::Prefab(self.prefab_id),
                from,
                to,
            )));
        } else {
            push_command(Box::new(BatchMoveEntitiesCmd::new(
                moves,
                EditorMode::Prefab(self.prefab_id),
            )));
        }
    }
}
