// editor/src/room/room_editor.rs
use crate::app::EditorCameraController;
use crate::app::SubEditor;
use crate::canvas::grid;
use crate::canvas::grid_shader::GridRenderer;
use crate::commands::room::*;
use crate::editor_assets::assets::*;
use crate::editor_global::*;
use crate::gui::inspector::inspector_panel::InspectorPanel;
use crate::gui::modal::is_modal_open;
use crate::gui::mode_selector::*;
use crate::gui::panels::panel_manager::is_mouse_over_panel;
use crate::room::drawing::*;
use crate::room::selection::PreCopyDragState;
use crate::shared::selection::draw_selection_box;
use crate::tilemap::tilemap_editor::*;
use crate::world::coord;
use bishop::prelude::*;
use engine_core::prelude::*;
use once_cell::sync::Lazy;
use std::collections::HashSet;
use strum::IntoEnumIterator;
use strum_macros::EnumIter;

#[derive(Clone, Copy, PartialEq, EnumIter)]
pub enum RoomEditorMode {
    Scene,
    Tilemap,
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

pub struct RoomEditor {
    pub mode: RoomEditorMode,
    pub mode_selector: ModeSelector<RoomEditorMode>,
    pub tilemap_editor: TileMapEditor,
    pub inspector: InspectorPanel,
    pub selected_entities: HashSet<Entity>,
    pub(crate) active_rects: Vec<Rect>,
    pub(crate) show_grid: bool,
    pub(crate) drag_offset: Vec2,
    pub(crate) dragging: bool,
    /// Stores for all dragged entities.
    pub(crate) drag_start_positions: Vec<(Entity, Vec2)>,
    /// The entity that was clicked to start the selection drag.
    pub(crate) drag_anchor_entity: Option<Entity>,
    initialized: bool,
    pub create_entity_requested: bool,
    pub request_play: bool,
    pub view_preview: bool,
    pub(crate) preview_camera_id: Option<usize>,
    /// Start position of the box selection in world coordinates.
    pub(crate) box_select_start: Option<Vec2>,
    /// Whether box selection is currently active.
    pub(crate) box_select_active: bool,
    /// Whether current drag is an alt+drag copy operation.
    pub(crate) alt_copy_mode: bool,
    /// Entities created during alt+drag copy for undo command.
    pub(crate) alt_copied_entities: Vec<Entity>,
    /// Original drag state before entering copy mode (for reverting on alt release).
    pub(crate) pre_copy_drag_state: Option<PreCopyDragState>,
    /// The very first start positions when drag began (for undo command).
    pub(crate) drag_initial_start_positions: Vec<(Entity, Vec2)>,
    /// Current sub-mode for tilemap editing.
    pub(crate) tilemap_sub_mode: TilemapEditorMode,
    /// Rect of the sub-mode strip for UI tracking.
    pub(crate) sub_mode_rect: Option<Rect>,
}

impl RoomEditor {
    pub fn new() -> Self {
        let mode = RoomEditorMode::Scene;

        Self {
            mode: RoomEditorMode::Scene,
            mode_selector: ModeSelector {
                current: mode,
                options: *ALL_MODES,
            },
            tilemap_editor: TileMapEditor::new(),
            inspector: InspectorPanel::new(),
            selected_entities: HashSet::new(),
            active_rects: Vec::new(),
            show_grid: true,
            drag_offset: Vec2::ZERO,
            dragging: false,
            drag_start_positions: Vec::new(),
            drag_anchor_entity: None,
            initialized: false,
            preview_camera_id: None,
            create_entity_requested: false,
            request_play: false,
            view_preview: false,
            box_select_start: None,
            box_select_active: false,
            alt_copy_mode: false,
            alt_copied_entities: Vec::new(),
            pre_copy_drag_state: None,
            drag_initial_start_positions: Vec::new(),
            tilemap_sub_mode: TilemapEditorMode::Tiles,
            sub_mode_rect: None,
        }
    }

    pub async fn update(
        &mut self,
        ctx: &mut WgpuContext,
        camera: &mut Camera2D,
        room_id: RoomId,
        ecs: &mut Ecs,
        current_world: &mut World,
        asset_manager: &mut AssetManager,
    ) {
        let grid_size = current_world.grid_size;

        let other_bounds: Vec<(Vec2, Vec2)> = current_world
            .rooms
            .iter()
            .filter(|r| r.id != room_id)
            .map(|r| (r.position, r.size))
            .collect();

        let adjacent_exits: Vec<(Vec2, ExitDirection)> = {
            let current_room = current_world.rooms.iter().find(|r| r.id == room_id);

            match current_room {
                Some(target) => current_world
                    .rooms
                    .iter()
                    .filter(|r| r.id != room_id)
                    .flat_map(|adj| adj.exits_facing_room(target, grid_size))
                    .collect(),
                None => vec![],
            }
        };

        let room = current_world
            .rooms
            .iter_mut()
            .find(|r| r.id == room_id)
            .expect("Could not find room in world.");

        if ctx.is_mouse_button_pressed(MouseButton::Left) && !self.should_block_canvas(ctx) {
            clear_all_input_focus();
        }

        if !self.initialized {
            EditorCameraController::reset_room_editor_camera(ctx, camera, room, grid_size);
            self.initialized = true;
        }

        self.handle_mouse_cursor(ctx);

        // Click-selection
        let mouse_screen: Vec2 = ctx.mouse_position().into();

        let mut ui_was_clicked = false;

        let delta_time = ctx.get_frame_time();

        update_animation_sytem(ctx, ecs, asset_manager, delta_time, room.id).await;

        match self.mode {
            RoomEditorMode::Tilemap => {
                // Sync sub-mode and UI rect to tilemap editor
                self.tilemap_editor.mode = self.tilemap_sub_mode;
                self.tilemap_editor.sub_mode_rect = self.sub_mode_rect;

                self.tilemap_editor
                    .update(
                        ctx,
                        asset_manager,
                        camera,
                        room,
                        &other_bounds,
                        &adjacent_exits,
                        grid_size,
                    )
                    .await;
            }
            RoomEditorMode::Scene => {
                if self.ui_was_clicked(ctx) {
                    ui_was_clicked = true;
                }

                let drag_handled = self.handle_selection(
                    ctx,
                    room.id,
                    camera,
                    ecs,
                    asset_manager,
                    mouse_screen,
                    ui_was_clicked,
                    grid_size,
                );

                if !drag_handled {
                    self.handle_keyboard_move(ctx, ecs, room.id);
                }

                // Handle batch delete when multiple entities selected
                if self.selected_entities.len() > 1 && Controls::delete(ctx) && !input_is_focused()
                {
                    let entities: Vec<Entity> = self.selected_entities.iter().copied().collect();
                    push_command(Box::new(BatchDeleteEntitiesCmd::new(entities, room.id)));
                }

                // Copy multiple selected entities
                if Controls::copy(ctx) && self.selected_entities.len() > 1 && !input_is_focused() {
                    let entities: Vec<Entity> = self.selected_entities.iter().copied().collect();
                    copy_entities(ecs, &entities);
                }

                // Create a new entity if create was pressed
                if self.create_entity_requested && self.inspector.target.is_none() {
                    // Build the entity
                    let entity = ecs
                        .create_entity()
                        .with(Transform {
                            position: room.position,
                            ..Default::default()
                        })
                        .with(CurrentRoom(room.id))
                        .with(Name("Entity".to_string()))
                        .finish();

                    // Immediately select it so the inspector shows the newly-created entity
                    self.selected_entities.clear();
                    self.selected_entities.insert(entity);
                    self.create_entity_requested = false;
                }

                // If exactly one entity is selected, show the inspector
                if self.selected_entities.len() == 1 {
                    let entity = *self.selected_entities.iter().next().unwrap();
                    self.inspector.set_target(Some(entity));
                } else {
                    self.inspector.set_target(None);
                }

                // If target was cleared by inspector, sync selection
                if self.inspector.target.is_none() && self.selected_entities.len() == 1 {
                    self.selected_entities.clear();
                }
            }
        }

        self.handle_shortcuts(ctx, camera, room, grid_size, ecs);
    }

    pub async fn draw(
        &mut self,
        ctx: &mut WgpuContext,
        camera: &Camera2D,
        room_id: RoomId,
        game: &mut Game,
        render_system: &mut RenderSystem,
        grid_renderer: &GridRenderer,
    ) {
        self.request_play = false; // This is very important
        self.active_rects.clear();

        let mut game_ctx = game.ctx_mut();
        let grid_size = game_ctx.cur_world.grid_size;

        // Panel rect for inspector and tilemap editor.
        const INSPECTOR_W: f32 = 325.0;
        let inspector_rect = Rect::new(
            ctx.screen_width() - INSPECTOR_W,
            0.0,
            INSPECTOR_W,
            ctx.screen_height(),
        );

        match self.mode {
            RoomEditorMode::Tilemap => {
                let Some(room) = game_ctx.cur_world.current_room_mut() else {
                    return;
                };

                let ecs = &mut *game_ctx.ecs;
                let asset_manager = &mut *game_ctx.asset_manager;

                self.tilemap_editor.tilemap_panel.set_rect(inspector_rect);
                self.tilemap_editor
                    .draw(ctx, camera, room, asset_manager, ecs, grid_size)
                    .await;

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
                    grid_size,
                    self.preview_camera_id,
                );

                let render_cam = if self.view_preview && room_camera.is_some() {
                    room_camera.as_ref().map(|c| &c.camera).unwrap_or(camera)
                } else {
                    camera
                };

                self.inspector.set_rect(inspector_rect);

                if self.view_preview {
                    render_system.resize_for_camera(render_cam.zoom);
                    render_system.begin_scene(ctx);
                } else {
                    render_system.resize_to_window(ctx);
                }

                // Draws everything in the room. Same implementation as the game.
                render_room(ctx, &mut game_ctx, render_system, render_cam, 0.0, None);

                if self.view_preview {
                    render_system.end_scene(ctx);
                    render_system.present_game(ctx);
                }

                if !self.view_preview {
                    let Some(room) = game_ctx.cur_world.current_room_mut() else {
                        return;
                    };

                    ctx.set_camera(camera);

                    if self.show_grid {
                        grid::draw_grid(ctx, grid_renderer, camera, grid_size);
                    }

                    let ecs = &*game_ctx.ecs;
                    let asset_manager = &mut *game_ctx.asset_manager;

                    draw_exit_placeholders(ctx, &room.exits, room.position, grid_size);
                    draw_camera_placeholders(ctx, ecs, room_id, grid_size);
                    draw_light_placeholders(ctx, ecs, room_id, grid_size);
                    draw_glow_placeholders(ctx, ecs, asset_manager, room_id, grid_size);
                    draw_interactable_ranges(ctx, ecs, room_id, grid_size);

                    // Highlight all selected entities and draw their overlays
                    for &selected_entity in &self.selected_entities {
                        if !is_pure_placeholder(ecs, selected_entity) {
                            highlight_selected_entity(
                                ctx,
                                ecs,
                                selected_entity,
                                asset_manager,
                                Color::YELLOW,
                                grid_size,
                            );
                        }
                        self.draw_camera_viewport(ctx, camera, ecs, selected_entity, room_id);
                        draw_pivot_marker(ctx, ecs, selected_entity);
                    }

                    // Draw collider only for single selection
                    if let Some(selected_entity) = self.single_selected_entity() {
                        draw_collider(ctx, ecs, selected_entity);
                    }

                    // Draw box selection rectangle
                    if self.box_select_active {
                        if let Some(start) = self.box_select_start {
                            let mouse_world = coord::mouse_world_pos(ctx, camera);
                            draw_selection_box(ctx, start, mouse_world);
                        }
                    }
                }
            }
        }

        // Scene UI
        if !self.view_preview {
            self.draw_ui(ctx, &mut game_ctx, camera);
        }
    }

    pub fn reset(&mut self) {
        self.inspector.set_target(None);
        self.tilemap_editor.reset();
        self.mode = RoomEditorMode::Scene;
        self.mode_selector.current = RoomEditorMode::Scene;
        self.selected_entities.clear();
        self.initialized = false;
        self.request_play = false;
        self.view_preview = false;
        self.preview_camera_id = None;
        self.box_select_start = None;
        self.box_select_active = false;
        self.drag_start_positions.clear();
        self.drag_initial_start_positions.clear();
        self.drag_anchor_entity = None;
        self.alt_copy_mode = false;
        self.alt_copied_entities.clear();
        self.pre_copy_drag_state = None;
        self.tilemap_sub_mode = TilemapEditorMode::Tiles;
        self.sub_mode_rect = None;
    }

    /// Takes any pending toast message from the room/tilemap editor.
    pub fn take_pending_toast(&mut self) -> Option<&'static str> {
        self.tilemap_editor.take_pending_toast()
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
            || is_dropdown_open()
            || is_modal_open()
            || is_mouse_over_panel(ctx)
    }
}

/// A slice of all the modes.
static ALL_MODES: Lazy<&'static [RoomEditorMode]> =
    Lazy::new(|| Box::leak(Box::new(RoomEditorMode::iter().collect::<Vec<_>>())));
