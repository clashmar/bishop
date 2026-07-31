use crate::app::EditorCameraController;
use crate::app::EditorMode;
use crate::app::SubEditor;
use crate::canvas::grid;
use crate::canvas::grid_shader::GridRenderer;
use crate::commands::room::*;
use crate::commands::scene::CreateSceneEntityCmd;
use crate::editor_assets::assets::*;
use crate::editor_global::*;
use crate::gui::gui_constants::{self};
use crate::gui::inspector::shell::Inspector;
use crate::gui::mode_selector::*;
use crate::prefab::reconcile_recent_prefab_ids;
use crate::room::drawing::*;
use crate::room::interior_zone_edit::InteriorZoneEditorState;
use crate::room::layer_state::RoomLayerState;
use crate::room::selection::{DragState, can_select_entity_in_room_layer};
use crate::shared::input::{canvas_blocked_by_global_ui, shortcuts_blocked};
use crate::shared::scene_ui::inspector::{CreateRequest, PrefabActionRequest};
use crate::shared::selection::draw_selection_box;
use crate::prefab::palette::{PrefabPaletteState, PREFAB_PALETTE_RECENT_CAP};
use crate::storage::lua_stub_gen::collect_custom_event_tags;
use crate::tilemap::tilemap_editor::*;
use crate::world::coord;
use bishop::prelude::*;
use engine_core::animation::{update_animation_sytem};
use engine_core::assets::*;
use engine_core::camera::get_room_camera_by_id;
use engine_core::controls::{Controls};
use engine_core::ecs::*;
use engine_core::game::Game;
use engine_core::rendering::{render_room, RenderSystem, RoomRenderState};
use engine_core::worlds::*;
use widgets::*;
use once_cell::sync::Lazy;
use std::collections::HashSet;
use strum::IntoEnumIterator;
use strum_macros::EnumIter;

#[derive(Clone, Copy, Debug, PartialEq, EnumIter)]
pub enum RoomEditorMode {
    Scene,
    Tilemap,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum RoomSceneSubMode {
    Scene,
    Stamp,
    Zones,
}

pub(crate) static ROOM_SCENE_SUB_MODES: &[RoomSceneSubMode] =
    &[RoomSceneSubMode::Scene, RoomSceneSubMode::Stamp, RoomSceneSubMode::Zones];

#[derive(Clone, Copy)]
pub(crate) struct ActivePrefabStampState {
    pub(crate) available: bool,
    pub(crate) pivot: Pivot,
}

pub(crate) struct RoomEditorUpdateState<'a> {
    pub(crate) room_id: RoomId,
    pub(crate) ecs: &'a mut Ecs,
    pub(crate) current_world: &'a mut World,
    pub(crate) asset_registry: &'a mut AssetRegistry,
    pub(crate) sprite_manager: &'a mut SpriteManager,
    pub(crate) active_prefab_stamp: ActivePrefabStampState,
}

impl ModeInfo for RoomEditorMode {
    fn label(&self) -> &'static str {
        match self {
            RoomEditorMode::Scene => "Scene Editor: S",
            RoomEditorMode::Tilemap => "Tilemap Editor: T",
        }
    }
    fn icon(&self) -> &'static Texture2D {
        match self {
            RoomEditorMode::Scene => entity_icon(),
            RoomEditorMode::Tilemap => grid_icon(),
        }
    }
    fn shortcut(self) -> Option<fn(&WgpuContext) -> bool> {
        match self {
            RoomEditorMode::Scene => Some(Controls::s),
            RoomEditorMode::Tilemap => Some(Controls::t),
        }
    }
}

impl ModeInfo for RoomSceneSubMode {
    fn label(&self) -> &'static str {
        match self {
            RoomSceneSubMode::Scene => "Scene",
            RoomSceneSubMode::Stamp => "Stamp",
            RoomSceneSubMode::Zones => "Zones",
        }
    }

    fn icon(&self) -> &'static Texture2D {
        match self {
            RoomSceneSubMode::Scene => edit_icon(),
            RoomSceneSubMode::Stamp => select_icon(),
            RoomSceneSubMode::Zones => create_icon(),
        }
    }

    fn shortcut(self) -> Option<fn(&WgpuContext) -> bool> {
        None
    }
}

pub struct RoomEditor {
    pub mode: RoomEditorMode,
    pub mode_selector: ModeSelector<RoomEditorMode>,
    pub tilemap_editor: TileMapEditor,
    pub inspector: Inspector,
    pub active_layer_state: RoomLayerState,
    pub selected_entities: HashSet<Entity>,
    pub active_prefab_id: Option<PrefabId>,
    pub recent_prefab_ids: Vec<PrefabId>,
    pub(crate) scene_sub_mode: RoomSceneSubMode,
    pub(crate) active_rects: Vec<Rect>,
    pub(crate) show_grid: bool,
    pub(crate) drag_state: DragState,
    pub(crate) interior_zone_editor: InteriorZoneEditorState,
    pub create_request: Option<CreateRequest>,
    pub prefab_action_request: Option<PrefabActionRequest>,
    pub create_camera_request: Option<f32>,
    pub event_tags: Vec<String>,
    pub request_event_tags_refresh: bool,
    pub request_play: bool,
    pub view_preview: bool,
    pub(crate) preview_camera_id: Option<usize>,
    /// Current sub-mode for tilemap editing.
    pub(crate) tilemap_sub_mode: TilemapEditorMode,
    /// Rect of the sub-mode strip for UI tracking.
    pub(crate) sub_mode_rect: Option<Rect>,
}

impl RoomEditor {
    pub fn new() -> Self {
        let mode = RoomEditorMode::Scene;

        let mut inspector = Inspector::new();
        inspector.select_room();

        Self {
            mode: RoomEditorMode::Scene,
            mode_selector: ModeSelector {
                current: mode,
                options: *ALL_MODES,
            },
            tilemap_editor: TileMapEditor::new(),
            inspector,
            active_layer_state: RoomLayerState::default(),
            selected_entities: HashSet::new(),
            active_prefab_id: None,
            recent_prefab_ids: Vec::new(),
            scene_sub_mode: RoomSceneSubMode::Scene,
            active_rects: Vec::new(),
            show_grid: true,
            drag_state: DragState::default(),
            interior_zone_editor: InteriorZoneEditorState::default(),
            preview_camera_id: None,
            create_request: None,
            prefab_action_request: None,
            create_camera_request: None,
            event_tags: Vec::new(),
            request_event_tags_refresh: false,
            request_play: false,
            view_preview: false,
            tilemap_sub_mode: TilemapEditorMode::Tiles,
            sub_mode_rect: None,
        }
    }

    pub fn update(
        &mut self,
        ctx: &mut WgpuContext,
        camera: &mut Camera2D,
        state: RoomEditorUpdateState<'_>,
    ) {
        let RoomEditorUpdateState {
            room_id,
            ecs,
            current_world,
            asset_registry,
            sprite_manager,
            active_prefab_stamp,
        } = state;
        let grid_size = current_world.grid_size;

        let other_bounds: Vec<(Vec2, Vec2)> = current_world
            .rooms()
            .iter()
            .filter(|r| r.id != room_id)
            .map(|r| (r.position, r.size))
            .collect();

        let adjacent_exits: Vec<RoomFacingExit> = {
            let current_room = current_world.get_room(room_id);

            match current_room {
                Some(target) => current_world
                    .rooms()
                    .iter()
                    .filter(|r| r.id != room_id)
                    .flat_map(|adj| adj.exits_facing_room(target, grid_size))
                    .collect(),
                None => vec![],
            }
        };

        let room_has_back_layer = current_world
            .get_room(room_id)
            .is_some_and(|room| room.current_variant().layers.back.is_some());
        if !room_has_back_layer && self.scene_sub_mode == RoomSceneSubMode::Zones {
            self.reset_scene_sub_mode();
        }
        self.sync_active_layer_for_room(ecs, room_id, room_has_back_layer);

        let world_id = current_world.id;
        let room = current_world
            .rooms_mut()
            .iter_mut()
            .find(|r| r.id == room_id)
            .expect("Could not find room in world.");

        if ctx.is_mouse_button_pressed(MouseButton::Left) && !self.should_block_canvas(ctx) {
            clear_all_input_focus();
        }

        self.handle_mouse_cursor(ctx);

        let delta_time = ctx.get_frame_time();

        update_animation_sytem(
            ctx,
            ecs,
            asset_registry,
            sprite_manager,
            delta_time,
            room.id,
        );

        match self.mode {
            RoomEditorMode::Tilemap => {
                self.inspector.select_tilemap();
                self.tilemap_editor.mode = self.tilemap_sub_mode;
                self.tilemap_editor.sub_mode_rect = self.sub_mode_rect;
                self.tilemap_editor
                    .set_selected_tile(self.inspector.selected_tile_brush());
                self.tilemap_editor.sync_adjacent_exits(&adjacent_exits);
                self.tilemap_editor.update(
                    ctx,
                    self.inspector.is_mouse_over(ctx),
                    camera,
                    room,
                    ecs,
                    &other_bounds,
                    grid_size,
                );
            }
            RoomEditorMode::Scene => {
                let zone_handled = self.handle_interior_zones(
                    ctx,
                    camera,
                    world_id,
                    room,
                    grid_size,
                );
                if self.scene_sub_mode != RoomSceneSubMode::Zones {
                    let stamp_handled = self
                        .handle_prefab_stamp(ctx, camera, room.id, grid_size, active_prefab_stamp);
                    let drag_handled = stamp_handled
                        || self.handle_selection(
                            ctx,
                            room.id,
                            camera,
                            ecs,
                            sprite_manager,
                            grid_size,
                        );

                    if !drag_handled {
                        self.handle_keyboard_move(ctx, ecs, room.id);
                    }

                    // Handle batch delete when multiple entities selected
                    if self.selected_entities.len() > 1
                        && Controls::delete(ctx)
                        && !shortcuts_blocked()
                    {
                        let entities: Vec<Entity> = self.selected_entities.iter().copied().collect();
                        push_command(Box::new(BatchDeleteEntitiesCmd::new(
                            entities,
                            EditorMode::Room(room_id),
                        )));
                    }

                    // Copy multiple selected entities
                    if Controls::copy(ctx)
                        && self.selected_entities.len() > 1
                        && !shortcuts_blocked()
                    {
                        let entities: Vec<Entity> = self.selected_entities.iter().copied().collect();
                        copy_entities(ecs, &entities);
                    }
                } else if !zone_handled {
                    self.inspector.select_room();
                }

                // Create a new entity if create was pressed
                if let Some(create_request) = self.create_request.take() {
                    push_command(Box::new(CreateSceneEntityCmd::new_room_entity(
                        room.id,
                        self.active_layer_state.active_layer,
                        room.position,
                        create_request.parent,
                    )));
                }

                // Create a new camera if create_camera_request was emitted
                if let Some(cam_grid_size) = self.create_camera_request.take() {
                    push_command(Box::new(CreateSceneEntityCmd::new_room_camera(
                        room.id,
                        self.active_layer_state.active_layer,
                        room.position,
                        cam_grid_size,
                    )));
                }

                // If target was cleared by inspector, sync selection.
                if self.scene_sub_mode != RoomSceneSubMode::Zones
                    && !self.inspector.has_target()
                    && self.selected_entities.len() == 1
                {
                    self.clear_selection();
                }
            }
        }

        self.handle_shortcuts(ctx, camera, room, world_id, grid_size, ecs);

        current_world.link_all_room_exits();
    }

    pub(crate) fn prefab_palette_state(&self) -> PrefabPaletteState {
        PrefabPaletteState {
            active_prefab_id: self.active_prefab_id,
            recent_prefab_ids: self.recent_prefab_ids.clone(),
        }
    }

    pub(crate) fn load_prefab_palette_state(
        &mut self,
        prefab_manager: &PrefabManager,
        state: PrefabPaletteState,
    ) {
        self.active_prefab_id = state
            .active_prefab_id
            .filter(|prefab_id| prefab_manager.prefabs.contains_key(prefab_id));
        self.recent_prefab_ids =
            reconcile_recent_prefab_ids(state.recent_prefab_ids, prefab_manager);
        self.set_scene_sub_mode(RoomSceneSubMode::Scene);
    }

    pub(crate) fn reconcile_prefab_palette(&mut self, prefab_manager: &PrefabManager) {
        self.recent_prefab_ids =
            reconcile_recent_prefab_ids(self.recent_prefab_ids.clone(), prefab_manager);

        if self
            .active_prefab_id
            .is_some_and(|prefab_id| !prefab_manager.prefabs.contains_key(&prefab_id))
        {
            self.active_prefab_id = self.recent_prefab_ids.first().copied();
        }
    }

    pub(crate) fn activate_prefab(&mut self, prefab_id: PrefabId) {
        self.active_prefab_id = Some(prefab_id);
        self.record_recent_prefab(prefab_id);
        self.mode = RoomEditorMode::Scene;
        self.mode_selector.current = RoomEditorMode::Scene;
        self.set_preview_enabled(false);
        self.set_scene_sub_mode(RoomSceneSubMode::Stamp);
    }

    pub(crate) fn record_recent_prefab(&mut self, prefab_id: PrefabId) {
        self.recent_prefab_ids.retain(|id| *id != prefab_id);
        self.recent_prefab_ids.insert(0, prefab_id);
        self.recent_prefab_ids.truncate(PREFAB_PALETTE_RECENT_CAP);
    }

    pub(crate) fn reset_scene_sub_mode(&mut self) -> bool {
        let was_active = self.scene_sub_mode != RoomSceneSubMode::Scene;
        self.scene_sub_mode = RoomSceneSubMode::Scene;
        self.interior_zone_editor.clear();
        was_active
    }

    pub(crate) fn set_mode(&mut self, mode: RoomEditorMode) {
        if self.mode != mode {
            self.reset_scene_sub_mode();
        }
        self.mode = mode;
        self.mode_selector.current = mode;

        match mode {
            RoomEditorMode::Tilemap => self.inspector.select_tilemap(),
            RoomEditorMode::Scene => self.sync_inspector_to_selection(),
        }
    }

    pub(crate) fn prune_selection_to_active_layer(&mut self, ecs: &Ecs, room_id: RoomId) {
        let active_layer = self.active_layer_state.active_layer;
        let selected_count = self.selected_entities.len();
        self.selected_entities
            .retain(|entity| can_select_entity_in_room_layer(ecs, *entity, room_id, active_layer));
        if self.selected_entities.len() != selected_count {
            self.sync_inspector_to_selection();
        }
    }

    pub(crate) fn set_active_layer(&mut self, ecs: &Ecs, room_id: RoomId, layer: RoomLayer) {
        if self.active_layer_state.active_layer != layer {
            self.disable_active_edit_modes();
            self.active_layer_state.active_layer = layer;
        }
        self.tilemap_editor.active_layer = self.active_layer_state.active_layer;
        self.prune_selection_to_active_layer(ecs, room_id);
    }

    pub(crate) fn toggle_active_layer(
        &mut self,
        ecs: &Ecs,
        room_id: RoomId,
        has_back_layer: bool,
    ) {
        if !has_back_layer {
            self.set_active_layer(ecs, room_id, RoomLayer::Front);
            return;
        }

        let mut next_state = self.active_layer_state;
        next_state.toggle();
        self.set_active_layer(ecs, room_id, next_state.active_layer);
    }

    pub(crate) fn sync_active_layer_for_room(
        &mut self,
        ecs: &Ecs,
        room_id: RoomId,
        has_back_layer: bool,
    ) {
        if has_back_layer {
            self.tilemap_editor.active_layer = self.active_layer_state.active_layer;
            self.prune_selection_to_active_layer(ecs, room_id);
        } else {
            self.set_active_layer(ecs, room_id, RoomLayer::Front);
        }
    }

    pub(crate) fn set_preview_enabled(&mut self, enabled: bool) {
        if self.view_preview != enabled {
            self.reset_scene_sub_mode();
        }
        self.view_preview = enabled;
        if !enabled {
            self.preview_camera_id = None;
        }
    }

    pub(crate) fn set_tilemap_sub_mode(&mut self, mode: TilemapEditorMode) {
        if self.tilemap_sub_mode != mode {
            self.reset_scene_sub_mode();
        }
        self.tilemap_sub_mode = mode;
        self.tilemap_editor.mode = mode;
    }

    pub(crate) fn set_scene_sub_mode(&mut self, mode: RoomSceneSubMode) {
        self.scene_sub_mode = mode;
        if mode == RoomSceneSubMode::Zones {
            self.disable_active_edit_modes();
            self.drag_state = DragState::default();
            self.inspector.select_room();
        } else {
            self.interior_zone_editor.clear();
            self.sync_inspector_to_selection();
        }
    }

    pub(crate) fn toggle_zone_sub_mode(&mut self) {
        self.set_mode(RoomEditorMode::Scene);
        if self.scene_sub_mode == RoomSceneSubMode::Zones {
            self.set_scene_sub_mode(RoomSceneSubMode::Scene);
        } else {
            self.set_scene_sub_mode(RoomSceneSubMode::Zones);
        }
    }

    pub(crate) fn active_prefab_snap_pivot(&self, prefab_manager: &PrefabManager) -> Pivot {
        let Some(prefab_id) = self.active_prefab_id else {
            return Pivot::BottomCenter;
        };
        let Some(prefab) = prefab_manager.prefabs.get(&prefab_id) else {
            return Pivot::BottomCenter;
        };
        let Some(root) = prefab
            .nodes
            .iter()
            .find(|node| node.node_id == prefab.root_node_id)
        else {
            return Pivot::BottomCenter;
        };

        root.components
            .iter()
            .find(|component| component.type_name == comp_type_name::<Transform>())
            .and_then(|component| ron::from_str::<Transform>(&component.ron).ok())
            .map(|transform| transform.pivot)
            .unwrap_or(Pivot::BottomCenter)
    }

    pub fn draw(
        &mut self,
        ctx: &mut WgpuContext,
        camera: &Camera2D,
        room_id: RoomId,
        game: &mut Game,
        render_system: &mut RenderSystem,
        grid_renderer: &GridRenderer,
    ) {
        self.request_play = false; // This is very important
        self.request_event_tags_refresh = false;
        self.event_tags = collect_custom_event_tags(game);
        self.active_rects.clear();
        let active_prefab = self
            .active_prefab_id
            .and_then(|prefab_id| game.prefab_manager.prefabs.get(&prefab_id).cloned());
        let active_prefab_snap_pivot = self.active_prefab_snap_pivot(&game.prefab_manager);
        {
            let mut game_ctx = game.ctx_mut();
            let Some(grid_size) = game_ctx.world.as_deref().map(|world| world.grid_size) else {
                return;
            };

            // Right-side inspector rect
            let inspector_rect = Rect::new(
                ctx.screen_width() - gui_constants::inspector::WIDTH,
                0.0,
                gui_constants::inspector::WIDTH,
                ctx.screen_height(),
            );

            match self.mode {
                RoomEditorMode::Tilemap => {
                    self.inspector.set_rect(inspector_rect);
                    self.inspector.select_tilemap();

                    let Some(room) = game_ctx
                        .world
                        .as_deref_mut()
                        .and_then(World::current_room_mut)
                    else {
                        return;
                    };

                    let ecs = &*game_ctx.ecs;
                    let tile_registry = &*game_ctx.tile_registry;
                    let sprite_manager = &mut *game_ctx.sprite_manager;

                    self.tilemap_editor.draw(
                        ctx,
                        camera,
                        room,
                        (tile_registry, sprite_manager),
                        ecs,
                        grid_size,
                    );

                    ctx.set_camera(camera);
                    if self.show_grid {
                        grid::draw_grid(ctx, grid_renderer, camera, grid_size);
                    }
                }
                RoomEditorMode::Scene => {
                    let room_camera = get_room_camera_by_id(
                        ctx,
                        &*game_ctx.ecs,
                        room_id,
                        self.active_layer_state.active_layer,
                        grid_size,
                        self.preview_camera_id,
                    );

                    let view_preview = self.view_preview;
                    let render_cam = if view_preview && room_camera.is_some() {
                        room_camera.as_ref().map(|c| &c.camera).unwrap_or(camera)
                    } else {
                        camera
                    };

                    self.inspector.set_rect(inspector_rect);

                    if view_preview {
                        render_system.resize_for_camera(render_cam.zoom);
                        render_system.begin_scene(ctx);
                    } else {
                        render_system.resize_to_window(ctx);
                    }

                    render_room(
                        ctx,
                        &mut game_ctx,
                        render_cam,
                        RoomRenderState {
                            current_layer: self.active_layer_state.active_layer,
                            viewpoint_position: Some(render_cam.target),
                        },
                        0.0,
                        None,
                    );

                    if view_preview {
                        render_system.end_scene(ctx);
                        render_system.present_game(ctx);
                    }

                    if !view_preview {
                        let Some(room) = game_ctx
                            .world
                            .as_deref_mut()
                            .and_then(World::current_room_mut)
                        else {
                            return;
                        };

                        ctx.set_camera(camera);

                        if self.show_grid {
                            grid::draw_grid(ctx, grid_renderer, camera, grid_size);
                        }

                        let ecs = &*game_ctx.ecs;
                        let asset_registry = &mut *game_ctx.asset_registry;
                        let sprite_manager = &mut *game_ctx.sprite_manager;

                        draw_exit_placeholders(
                            ctx,
                            &room.exits,
                            room.position,
                            self.active_layer_state.active_layer,
                            grid_size,
                        );
                        draw_entity_placeholders(
                            ctx,
                            ecs,
                            sprite_manager,
                            room_id,
                            self.active_layer_state.active_layer,
                            grid_size,
                        );
                        draw_entity_interaction_guides_in_room(
                            ctx,
                            ecs,
                            room_id,
                            self.active_layer_state.active_layer,
                            grid_size,
                        );
                        self.draw_interior_zones_overlay(ctx, room, grid_size);
                        if self.scene_sub_mode == RoomSceneSubMode::Stamp
                            && !self.should_block_canvas(ctx)
                        {
                            if let Some(prefab) = active_prefab.as_ref() {
                                draw_prefab_stamp_ghost(
                                    ctx,
                                    camera,
                                    asset_registry,
                                    sprite_manager,
                                    prefab,
                                    grid_size,
                                    active_prefab_snap_pivot,
                                );
                            }
                        }

                        if self.scene_sub_mode != RoomSceneSubMode::Zones {
                            for &selected_entity in &self.selected_entities {
                                if !is_pure_placeholder(ecs, selected_entity) {
                                    highlight_selected_entity(
                                        ctx,
                                        ecs,
                                        selected_entity,
                                        sprite_manager,
                                        grid_size,
                                    );
                                }
                                self.draw_camera_viewports(
                                    ctx,
                                    camera,
                                    ecs,
                                    selected_entity,
                                    room_id,
                                    self.active_layer_state.active_layer,
                                );
                                draw_pivot_marker(ctx, ecs, selected_entity);
                            }

                            if let Some(selected_entity) = self.single_selected_entity() {
                                draw_editor_collider(ctx, ecs, selected_entity, grid_size);
                            }

                            if self.drag_state.box_select_active {
                                if let Some(start) = self.drag_state.box_select_start {
                                    let mouse_world = coord::mouse_world_pos(ctx, camera);
                                    draw_selection_box(ctx, start, mouse_world, grid_size);
                                }
                            }
                        }
                    }
                }
            }

            if !self.view_preview {
                self.draw_ui(ctx, &mut game_ctx, camera);
            }
        }
    }

    /// Resets the camera to frame the given room.
    pub fn init_camera(ctx: &WgpuContext, camera: &mut Camera2D, room: &Room, grid_size: f32) {
        EditorCameraController::reset_room_editor_camera(ctx, camera, room, grid_size);
    }

    pub fn reset(&mut self) {
        self.inspector.select_room();
        self.tilemap_editor.reset();
        self.reset_scene_sub_mode();
        self.mode = RoomEditorMode::Scene;
        self.mode_selector.current = RoomEditorMode::Scene;
        self.selected_entities.clear();
        self.active_layer_state = RoomLayerState::default();
        self.create_request = None;
        self.prefab_action_request = None;
        self.create_camera_request = None;
        self.request_play = false;
        self.view_preview = false;
        self.preview_camera_id = None;
        self.drag_state = DragState::default();
        self.interior_zone_editor.clear();
        self.tilemap_sub_mode = TilemapEditorMode::Tiles;
        self.sub_mode_rect = None;
    }
}

impl SubEditor for RoomEditor {
    fn active_rects(&self) -> &[Rect] {
        &self.active_rects
    }

    fn should_block_canvas(&self, ctx: &WgpuContext) -> bool {
        let mouse_screen: Vec2 = ctx.mouse_position().into();
        self.active_rects.iter().any(|r| r.contains(mouse_screen))
            || self.sub_mode_rect.is_some_and(|r| r.contains(mouse_screen))
            || self.inspector.is_mouse_over(ctx)
            || canvas_blocked_by_global_ui(ctx)
    }
}

/// A slice of all the modes.
static ALL_MODES: Lazy<&'static [RoomEditorMode]> =
    Lazy::new(|| Box::leak(Box::new(RoomEditorMode::iter().collect::<Vec<_>>())));

#[cfg(test)]
#[path = "tests/room_editor_tests.rs"]
mod tests;
