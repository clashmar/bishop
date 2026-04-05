use crate::app::EditorMode;
use crate::commands::room::{BatchMoveEntitiesCmd, MoveEntityCmd};
use crate::editor_global::push_command;
use crate::prefab::prefab_editor::{PrefabDragState, PrefabEditor, PREFAB_EDITOR_GRID_SIZE};
use crate::prefab::selection::is_prefab_entity;
use crate::room::entity_hitbox;
use crate::world::coord;
use bishop::prelude::*;
use engine_core::prelude::*;

impl PrefabEditor {
    pub(crate) fn handle_canvas_move(
        &mut self,
        ctx: &WgpuContext,
        camera: &Camera2D,
        ecs: &mut Ecs,
        asset_manager: &mut AssetManager,
    ) -> bool {
        let shift_held =
            ctx.is_key_down(KeyCode::LeftShift) || ctx.is_key_down(KeyCode::RightShift);
        let mouse_screen: Vec2 = ctx.mouse_position().into();
        let mouse_world = coord::mouse_world_pos(ctx, camera);

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

        if !ctx.is_mouse_button_pressed(MouseButton::Left) {
            return false;
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
                asset_manager,
                PREFAB_EDITOR_GRID_SIZE,
            );

            if hitbox.contains(mouse_screen) {
                let z = ecs.get_store::<Layer>().get(*entity).map_or(0, |layer| layer.z);
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
                self.start_drag(ecs, entity, mouse_world);
            }
            (false, None) => self.set_selected_entity(None),
            (true, None) => {}
        }

        self.drag_state.dragging
    }

    pub(crate) fn handle_keyboard_move(&mut self, ctx: &WgpuContext, ecs: &mut Ecs) {
        if self.drag_state.dragging || self.selected_entities.is_empty() || input_is_focused() {
            return;
        }

        let delta = get_omni_input_pressed(ctx);
        if delta.length_squared() == 0.0 {
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
