use bishop::prelude::*;
use engine_core::assets::SpriteManager;
use engine_core::controls::Controls;
use engine_core::ecs::*;
use engine_core::worlds::*;

use crate::app::EditorMode;
use crate::commands::room::{copy_entities, BatchDeleteEntitiesCmd};
use crate::commands::scene::CreateSceneEntityCmd;
use crate::editor_global::push_command;
use crate::room::room_editor::{ActivePrefabStampState, RoomEditor, RoomSceneSubMode};
use crate::shared::input::shortcuts_blocked;

impl RoomEditor {
    pub(crate) fn update_scene_mode(
        &mut self,
        ctx: &mut WgpuContext,
        camera: &mut Camera2D,
        world_id: WorldId,
        room: &mut Room,
        ecs: &mut Ecs,
        sprite_manager: &mut SpriteManager,
        active_prefab_stamp: ActivePrefabStampState,
        grid_size: f32,
    ) {
        let zone_handled = self.handle_interior_zones(ctx, camera, world_id, room, grid_size);
        if self.scene_sub_mode != RoomSceneSubMode::Zones {
            let stamp_handled =
                self.handle_prefab_stamp(ctx, camera, room.id, grid_size, active_prefab_stamp);
            let drag_handled =
                stamp_handled || self.handle_selection(ctx, room.id, camera, ecs, sprite_manager, grid_size);

            if !drag_handled {
                self.handle_keyboard_move(ctx, ecs, room.id);
            }

            if self.selected_entities.len() > 1 && Controls::delete(ctx) && !shortcuts_blocked() {
                let entities: Vec<Entity> = self.selected_entities.iter().copied().collect();
                push_command(Box::new(BatchDeleteEntitiesCmd::new(
                    entities,
                    EditorMode::Room(room.id),
                )));
            }

            if Controls::copy(ctx) && self.selected_entities.len() > 1 && !shortcuts_blocked() {
                let entities: Vec<Entity> = self.selected_entities.iter().copied().collect();
                copy_entities(ecs, &entities);
            }
        } else if !zone_handled {
            self.inspector.select_room();
        }

        if let Some(create_request) = self.create_request.take() {
            push_command(Box::new(CreateSceneEntityCmd::new_room_entity(
                room.id,
                self.active_layer_state.active_layer,
                room.position,
                create_request.parent,
            )));
        }

        if let Some(cam_grid_size) = self.create_camera_request.take() {
            push_command(Box::new(CreateSceneEntityCmd::new_room_camera(
                room.id,
                self.active_layer_state.active_layer,
                room.position,
                cam_grid_size,
            )));
        }

        if self.scene_sub_mode != RoomSceneSubMode::Zones
            && !self.inspector.has_target()
            && self.selected_entities.len() == 1
        {
            self.clear_selection();
        }
    }
}
