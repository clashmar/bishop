use crate::app::*;
use bishop::prelude::*;
use engine_core::worlds::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BackRoute {
    None,
    ExitWorld,
    ExitRoom,
    ExitMenu,
    ExitPrefab,
}

pub(crate) fn back_route_for_mode(mode: EditorMode) -> BackRoute {
    match mode {
        EditorMode::Game => BackRoute::None,
        EditorMode::World(_) => BackRoute::ExitWorld,
        EditorMode::Room(_) => BackRoute::ExitRoom,
        EditorMode::Menu => BackRoute::ExitMenu,
        EditorMode::Prefab(_) => BackRoute::ExitPrefab,
    }
}

pub(crate) fn menu_back_consumes_preview(view_preview: bool) -> bool {
    view_preview
}

impl Editor {
    pub(crate) fn navigate_back(&mut self, ctx: &mut WgpuContext) {
        match back_route_for_mode(self.mode) {
            BackRoute::None => {}
            BackRoute::ExitWorld => self.exit_world_mode(ctx),
            BackRoute::ExitRoom => {
                if let EditorMode::Room(room_id) = self.mode {
                    self.exit_room_mode(ctx, room_id);
                }
            }
            BackRoute::ExitMenu => self.exit_menu_mode(ctx),
            BackRoute::ExitPrefab => self.request_exit_prefab_mode(ctx),
        }
    }

    fn exit_menu_mode(&mut self, ctx: &WgpuContext) {
        if menu_back_consumes_preview(self.menu_editor.view_preview) {
            self.menu_editor.view_preview = false;
            return;
        }

        self.save_menus();
        let return_mode = self.return_mode.unwrap_or(EditorMode::Game);
        self.mode = return_mode;
        self.return_mode = None;

        match return_mode {
            EditorMode::Game => self.game_editor.init_camera(ctx, &mut self.camera, &mut self.game),
            EditorMode::World(id) => {
                if let Some(world) = self.game.get_world_mut(id) {
                    self.world_editor.init_camera(ctx, &mut self.camera, world);
                }
            }
            EditorMode::Room(id) => {
                let current_world = self.game.current_world();
                if let Some(room) = current_world.get_room(id) {
                    EditorCameraController::reset_room_editor_camera(
                        ctx,
                        &mut self.camera,
                        room,
                        current_world.grid_size,
                    );
                }
            }
            EditorMode::Prefab(_) | EditorMode::Menu => {}
        }
    }

    fn exit_world_mode(&mut self, ctx: &WgpuContext) {
        self.game_editor.init_camera(ctx, &mut self.camera, &mut self.game);
        self.cur_world_id = None;
        self.world_editor.reset();
        self.mode = EditorMode::Game;
        self.save();
    }

    fn exit_room_mode(&mut self, ctx: &WgpuContext, room_id: RoomId) {
        if self.room_editor.reset_scene_sub_mode() {
            self.save_prefab_palette_state();
            return;
        }

        let game_name = self.game.name.clone();
        let mut game_ctx = self.game.ctx_mut();
        let palette = &mut self.room_editor.tilemap_editor.tilemap_panel.palette;
        if let Err(e) = editor_storage::save_palette(palette, &game_name) {
            engine_core::omni_error!("Could not save tile palette: {e}");
        }

        let current_world = game_ctx
            .world
            .as_deref_mut()
            .expect("Current world id not present in game while in Room mode.");
        if let Some(room) = current_world.get_room(room_id) {
            self.world_editor
                .center_on_room(ctx, &mut self.camera, room, current_world.grid_size);
        }

        self.cur_room_id = None;
        self.room_editor.reset();
        self.mode = EditorMode::World(current_world.id);
        self.save_prefab_palette_state();
        self.save();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn back_route_maps_editor_modes_to_expected_navigation_paths() {
        assert_eq!(back_route_for_mode(EditorMode::Game), BackRoute::None);
        assert_eq!(back_route_for_mode(EditorMode::World(WorldId(1))), BackRoute::ExitWorld);
        assert_eq!(back_route_for_mode(EditorMode::Room(RoomId(1))), BackRoute::ExitRoom);
        assert_eq!(back_route_for_mode(EditorMode::Menu), BackRoute::ExitMenu);
        assert_eq!(back_route_for_mode(EditorMode::Prefab(PrefabId(7))), BackRoute::ExitPrefab);
    }

    #[test]
    fn menu_preview_consumes_back_before_mode_exit() {
        assert!(menu_back_consumes_preview(true));
        assert!(!menu_back_consumes_preview(false));
    }
}
