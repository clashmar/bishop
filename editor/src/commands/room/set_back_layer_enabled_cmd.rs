use crate::app::EditorMode;
use crate::commands::editor_command_manager::EditorCommand;
use crate::room::can_select_entity_in_room_layer;
use crate::with_editor;
use engine_core::ecs::{capture_subtree, restore_subtree, Ecs, GroupSnapshot, Parent};
use engine_core::worlds::{BackRoomLayer, RoomId, RoomLayer};
use std::collections::HashSet;

#[derive(Debug)]
pub struct SetBackLayerEnabledCmd {
    room_id: RoomId,
    enabled: bool,
    previous_back: Option<BackRoomLayer>,
    previous_active_layer: Option<RoomLayer>,
    removed_entities: Vec<GroupSnapshot>,
    state_captured: bool,
}

impl SetBackLayerEnabledCmd {
    pub fn new(room_id: RoomId, enabled: bool) -> Self {
        Self {
            room_id,
            enabled,
            previous_back: None,
            previous_active_layer: None,
            removed_entities: Vec::new(),
            state_captured: false,
        }
    }

    fn capture_state(&mut self) {
        if self.state_captured {
            return;
        }

        with_editor(|editor| {
            let Some(room) = editor.game.current_world().get_room(self.room_id) else {
                return;
            };

            self.previous_back = room.current_variant().layers.back.clone();
            if editor.cur_room_id == Some(self.room_id) {
                self.previous_active_layer = Some(editor.room_editor.active_layer_state.active_layer);
            }

            if self.enabled {
                self.state_captured = true;
                return;
            }

            let back_entities = editor
                .game
                .ecs
                .entities_in_room_layer(self.room_id, RoomLayer::Back)
                .iter()
                .copied()
                .collect::<HashSet<_>>();

            let root_entities = back_entities
                .iter()
                .copied()
                .filter(|entity| {
                    editor
                        .game
                        .ecs
                        .get::<Parent>(*entity)
                        .is_none_or(|parent| !back_entities.contains(&parent.0))
                })
                .collect::<Vec<_>>();

            for entity in root_entities {
                self.removed_entities
                    .push(capture_subtree(&mut editor.game.ecs, entity));
            }

            self.state_captured = true;
        });
    }

    fn apply_enabled(&self) {
        with_editor(|editor| {
            if let Some(room) = editor
                .game
                .current_world_mut()
                .and_then(|world| world.get_room_mut(self.room_id))
            {
                room.current_variant_mut().layers.back.get_or_insert_with(BackRoomLayer::default);
            }
        });
    }

    fn apply_disabled(&self) {
        with_editor(|editor| {
            let roots = self
                .removed_entities
                .iter()
                .filter_map(|snapshot| snapshot.first().map(|entity| entity.entity))
                .collect::<Vec<_>>();

            {
                if let Some(room) = editor
                    .game
                    .current_world_mut()
                    .and_then(|world| world.get_room_mut(self.room_id))
                {
                    room.current_variant_mut().layers.back = None;
                }
            }

            let mut ctx = editor.game.ctx_mut();
            for entity in roots {
                Ecs::remove_entity(&mut ctx, entity);
            }

            let ecs = &editor.game.ecs;
            if editor.cur_room_id == Some(self.room_id)
                && editor.room_editor.active_layer_state.active_layer == RoomLayer::Back
            {
                editor
                    .room_editor
                    .set_active_layer(ecs, self.room_id, RoomLayer::Front);
            } else {
                editor.room_editor.selected_entities.retain(|entity| {
                    can_select_entity_in_room_layer(
                        ecs,
                        *entity,
                        self.room_id,
                        editor.room_editor.active_layer_state.active_layer,
                    )
                });
                editor.room_editor.sync_inspector_to_selection();
            }
        });
    }

    fn restore_previous_state(&self) {
        with_editor(|editor| {
            {
                if let Some(room) = editor
                    .game
                    .current_world_mut()
                    .and_then(|world| world.get_room_mut(self.room_id))
                {
                    room.current_variant_mut().layers.back = self.previous_back.clone();
                }
            }

            let mut ctx = editor.game.ctx_mut();
            for snapshot in &self.removed_entities {
                restore_subtree(&mut ctx, snapshot);
            }

            let ecs = &editor.game.ecs;
            if let Some(layer) = self.previous_active_layer {
                editor.room_editor.set_active_layer(ecs, self.room_id, layer);
            } else {
                editor.room_editor.selected_entities.retain(|entity| {
                    can_select_entity_in_room_layer(
                        ecs,
                        *entity,
                        self.room_id,
                        editor.room_editor.active_layer_state.active_layer,
                    )
                });
                editor.room_editor.sync_inspector_to_selection();
            }
        });
    }
}

impl EditorCommand for SetBackLayerEnabledCmd {
    fn execute(&mut self) {
        self.capture_state();
        if self.enabled {
            self.apply_enabled();
        } else {
            self.apply_disabled();
        }
    }

    fn undo(&mut self) {
        self.restore_previous_state();
    }

    fn applies_in_mode(&self, current_mode: EditorMode) -> bool {
        current_mode == EditorMode::Room(self.room_id)
    }
}
