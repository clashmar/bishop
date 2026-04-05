// editor/src/room/shortcuts.rs
use crate::app::EditorMode;
use crate::app::EditorCameraController;
use crate::commands::room::*;
use crate::editor_global::push_command;
use crate::editor_global::with_panel_manager;
use crate::gui::mode_selector::ModeInfo;
use crate::gui::panels::hierarchy_panel::HIERARCHY_PANEL;
use crate::room::room_editor::*;
use bishop::prelude::*;
use engine_core::prelude::*;
use strum::IntoEnumIterator;

impl RoomEditor {
    pub(crate) fn handle_shortcuts(
        &mut self,
        ctx: &mut WgpuContext,
        camera: &mut Camera2D,
        room: &Room,
        grid_size: f32,
        ecs: &Ecs,
    ) {
        if input_is_focused() {
            return;
        }

        // Shortcuts for both tilemap and scene
        if Controls::g(ctx) {
            self.show_grid = !self.show_grid;
        }

        if Controls::r(ctx) {
            EditorCameraController::reset_room_editor_camera(ctx, camera, room, grid_size);
        }

        for mode in RoomEditorMode::iter() {
            if let Some(shortcut) = mode.shortcut() {
                if shortcut(ctx) {
                    self.mode = mode;
                    self.mode_selector.current = mode;
                    break;
                }
            }
        }

        match self.mode {
            RoomEditorMode::Tilemap => {}
            RoomEditorMode::Scene => {
                if Controls::v(ctx) {
                    self.view_preview = !self.view_preview;
                    if self.view_preview {
                        // If a single camera is selected, use it
                        let camera_id = self
                            .single_selected_entity()
                            .filter(|e| ecs.has::<RoomCamera>(*e))
                            .map(|e| e.0);

                        if camera_id.is_some() {
                            self.preview_camera_id = camera_id;
                        } else {
                            let first_camera =
                                get_next_room_camera(ctx, ecs, room.id, grid_size, None);
                            self.preview_camera_id = first_camera.map(|c| c.id);
                        }
                    } else {
                        self.preview_camera_id = None;
                    }
                }

                if self.view_preview && Controls::tab(ctx) {
                    let next_camera =
                        get_next_room_camera(ctx, ecs, room.id, grid_size, self.preview_camera_id);
                    self.preview_camera_id = next_camera.map(|c| c.id);
                }

                if Controls::paste(ctx) {
                    push_command(Box::new(PasteEntityCmd::new(EditorMode::Room(room.id))));
                }

                if Controls::h(ctx) {
                    with_panel_manager(|panel_manager| {
                        panel_manager.toggle(HIERARCHY_PANEL);
                    });
                }

                // Select all entities in room
                if Controls::select_all(ctx) {
                    self.select_all_in_room(ecs, room.id);
                }

                // Duplicate selected entities
                if Controls::duplicate(ctx) && !self.selected_entities.is_empty() {
                    let entities: Vec<Entity> = self.selected_entities.iter().copied().collect();
                    push_command(Box::new(DuplicateEntitiesCmd::new(
                        entities,
                        EditorMode::Room(room.id),
                    )));
                }
            }
        }
    }
}
