// editor/src/tilemap/tilemap_editor.rs
use crate::commands::room::*;
use crate::editor_assets::assets::*;
use crate::editor_global::{push_command, push_toast};
use crate::gui::gui_constants::MENU_PANEL_HEIGHT;
use crate::gui::menu_bar::draw_top_panel_full;
use crate::gui::modal::*;
use crate::gui::mode_selector::ModeInfo;
use crate::gui::panels::panel_manager::*;
use crate::room::drawing::*;
use crate::tilemap::resize_handle::*;
use crate::tilemap::tilemap_panel::TilemapPanel;
use bishop::prelude::*;
use engine_core::prelude::*;

fn thickness(grid_size: f32) -> f32 {
    (grid_size * 0.1).max(1.0)
}

#[derive(Clone, Copy, PartialEq)]
pub enum TilemapEditorMode {
    Tiles,
    Exits,
}

/// All tilemap editor sub-modes for the sub-mode selector.
pub static TILEMAP_SUB_MODES: &[TilemapEditorMode] =
    &[TilemapEditorMode::Tiles, TilemapEditorMode::Exits];

impl ModeInfo for TilemapEditorMode {
    fn label(&self) -> &'static str {
        match self {
            TilemapEditorMode::Tiles => "Tiles: T",
            TilemapEditorMode::Exits => "Exits: E",
        }
    }

    fn icon(&self) -> &'static Texture2D {
        match self {
            TilemapEditorMode::Tiles => tile_icon(),
            TilemapEditorMode::Exits => exit_icon(),
        }
    }

    fn shortcut(self) -> Option<fn(&WgpuContext) -> bool> {
        match self {
            TilemapEditorMode::Tiles => Some(Controls::t),
            TilemapEditorMode::Exits => Some(Controls::e),
        }
    }
}

pub struct TileMapEditor {
    pub mode: TilemapEditorMode,
    resize_handles: Vec<ResizeHandle>,
    active_handle_index: Option<usize>,
    preview_valid: bool,
    pub tilemap_panel: TilemapPanel,
    ui_was_clicked: bool,
    initialized: bool,
    adjacent_exits: Vec<(Vec2, ExitDirection)>,
    /// Rect of the sub-mode strip for UI blocking.
    pub sub_mode_rect: Option<Rect>,
}

impl TileMapEditor {
    pub fn new() -> Self {
        Self {
            mode: TilemapEditorMode::Tiles,
            resize_handles: Vec::new(),
            active_handle_index: None,
            preview_valid: true,
            tilemap_panel: TilemapPanel::new(),
            ui_was_clicked: false,
            initialized: false,
            adjacent_exits: Vec::new(),
            sub_mode_rect: None,
        }
    }

    pub fn update(
        &mut self,
        ctx: &WgpuContext,
        asset_manager: &mut AssetManager,
        camera: &mut Camera2D,
        room: &mut Room,
        other_bounds: &[(Vec2, Vec2)],
        adjacent_exits: &[(Vec2, ExitDirection)],
        grid_size: f32,
    ) {
        // Store adjacent exits for drawing
        self.adjacent_exits.clear();
        self.adjacent_exits.extend_from_slice(adjacent_exits);
        if !self.initialized {
            self.ui_was_clicked = true; // Stop any initial tile placements
            self.initialized = true;
        }

        self.tilemap_panel.update(asset_manager);

        // Only rebuild handles when not dragging (to preserve drag state)
        if self.active_handle_index.is_none() {
            let idx = room.current_variant_index();
            self.resize_handles =
                ResizeHandle::build_all(&room.variants[idx].tilemap, room.position, grid_size);
        }

        let mouse_screen: Vec2 = ctx.mouse_position().into();
        let screen_w = ctx.screen_width();
        let screen_h = ctx.screen_height();
        let mouse_world = camera.screen_to_world(mouse_screen, screen_w, screen_h);

        // Handle resize drag before tile placement
        let drag_active =
            self.handle_resize_drag(ctx, mouse_world, room, other_bounds, grid_size, room.id);

        // Consume UI clicks
        self.consume_ui_click(ctx, mouse_screen);

        if !self.ui_was_clicked && !drag_active {
            let room_position = room.position;
            let idx = room.current_variant_index();
            match self.mode {
                TilemapEditorMode::Tiles => self.handle_tile_placement(
                    ctx,
                    camera,
                    &mut room.variants[idx].tilemap,
                    room_position,
                    grid_size,
                ),
                TilemapEditorMode::Exits => self.handle_exit_placement(
                    ctx,
                    camera,
                    &room.variants[idx].tilemap,
                    &mut room.exits,
                    room_position,
                    grid_size,
                ),
            }
        }
    }

    /// Handle resize handle drag operations.
    /// Returns true if a drag is active (to block tile placement).
    fn handle_resize_drag(
        &mut self,
        ctx: &WgpuContext,
        mouse_world: Vec2,
        room: &Room,
        other_bounds: &[(Vec2, Vec2)],
        grid_size: f32,
        room_id: RoomId,
    ) -> bool {
        let idx = room.current_variant_index();
        let map = &room.variants[idx].tilemap;

        // Check for drag start
        if ctx.is_mouse_button_pressed(MouseButton::Left) && self.active_handle_index.is_none() {
            for (i, handle) in self.resize_handles.iter_mut().enumerate() {
                if handle.is_hovered(mouse_world) {
                    handle.begin_drag(mouse_world);
                    self.active_handle_index = Some(i);
                    self.ui_was_clicked = true;
                    break;
                }
            }
        }

        // Update active drag
        if let Some(handle_idx) = self.active_handle_index {
            // Cancel drag on any key press
            if Controls::any_key_pressed(ctx) {
                self.resize_handles[handle_idx].end_drag();
                self.active_handle_index = None;
                return false;
            }

            let handle = &mut self.resize_handles[handle_idx];
            let delta = handle.update_drag(mouse_world, grid_size);

            let preview_data = handle.compute_preview_bounds(room.position, room.size, grid_size);

            let resize_result = validate_resize(
                map,
                &room.exits,
                handle.side,
                delta,
                other_bounds,
                preview_data,
                grid_size,
            );

            self.preview_valid = matches!(resize_result, ResizeResult::Success);

            // Check for drag end
            if ctx.is_mouse_button_released(MouseButton::Left) {
                let should_apply = self.preview_valid && delta != 0;

                if should_apply {
                    let cmd = ResizeTilemapCmd::new(room_id, idx, handle.side, delta);
                    push_command(Box::new(cmd));
                }

                handle.end_drag();
                self.active_handle_index = None;

                if !should_apply {
                    if let Some(msg) = resize_result_message(resize_result) {
                        push_toast(msg, 2.5);
                    }
                }
            }

            return true;
        }

        false
    }

    fn consume_ui_click(&mut self, ctx: &WgpuContext, mouse_pos: Vec2) {
        if (ctx.is_mouse_button_pressed(MouseButton::Left)
            || ctx.is_mouse_button_pressed(MouseButton::Right))
            && self
                .tilemap_panel
                .handle_click(mouse_pos, self.tilemap_panel.rect)
        {
            self.ui_was_clicked = true;
            return;
        }

        // Unblock UI
        if ctx.is_mouse_button_released(MouseButton::Left)
            || !ctx.is_mouse_button_down(MouseButton::Left) && self.active_handle_index.is_none()
        {
            self.ui_was_clicked = false;
        }
    }

    fn handle_tile_placement(
        &mut self,
        ctx: &WgpuContext,
        camera: &Camera2D,
        map: &mut TileMap,
        room_position: Vec2,
        grid_size: f32,
    ) {
        let mouse_over_ui = self.is_mouse_over_ui(ctx, camera);
        let hover = self.get_hovered_tile(ctx, camera, map, room_position, grid_size);
        if mouse_over_ui || hover.is_none() {
            return;
        }

        let (x, y) = match hover.and_then(|h| h.as_usize()) {
            Some(coords) => coords,
            None => return,
        };

        // Remove
        if ctx.is_mouse_button_down(MouseButton::Left) && ctx.is_key_down(KeyCode::LeftAlt) {
            map.tiles.remove(&(x, y));
            return;
        }

        let def_id = match self.tilemap_panel.palette.selected_def_opt() {
            Some(d) => d,
            _ => return, // There is no tile to place
        };

        // Place
        if ctx.is_mouse_button_down(MouseButton::Left) {
            map.tiles.insert((x, y), def_id);
        }
    }

    fn handle_exit_placement(
        &mut self,
        ctx: &WgpuContext,
        camera: &Camera2D,
        map: &TileMap,
        exits: &mut Vec<Exit>,
        room_position: Vec2,
        grid_size: f32,
    ) {
        if self.is_mouse_over_ui(ctx, camera) {
            return;
        }

        if let Some(tile_pos) = self.get_hovered_edge(ctx, camera, map, room_position, grid_size) {
            let exit_direction = self.exit_direction_from_position(tile_pos, map);
            let exit_vec = vec2(tile_pos.x() as f32, tile_pos.y() as f32);

            if ctx.is_mouse_button_pressed(MouseButton::Left) {
                exits.push(Exit {
                    position: exit_vec,
                    direction: exit_direction,
                    target_room_id: None,
                });
            }

            if ctx.is_mouse_button_pressed(MouseButton::Right) {
                exits.retain(|exit| exit.position != exit_vec);
            }
        }
    }

    pub fn draw(
        &mut self,
        ctx: &mut WgpuContext,
        camera: &Camera2D,
        room: &mut Room,
        asset_manager: &mut AssetManager,
        ecs: &Ecs,
        grid_size: f32,
    ) {
        let variant_index = room.current_variant_index();
        let tilemap = &mut room.variants[variant_index].tilemap;
        let room_position = room.position;
        let room_id = room.id;
        let room_size = room.size;

        ctx.clear_background(Color::BLACK);
        ctx.set_camera(camera);
        tilemap.draw(ctx, asset_manager, room_position, grid_size);
        draw_exit_placeholders(ctx, &room.exits, room_position, grid_size);
        self.draw_adjacent_exits(ctx, grid_size);
        self.draw_hover_highlight(ctx, camera, tilemap, room_position, grid_size);

        if self.active_handle_index.is_some() {
            draw_all_camera_viewports(ctx, camera, ecs, room_id);
        }

        self.draw_ui(
            ctx,
            camera,
            asset_manager,
            tilemap,
            room_position,
            room_size,
            grid_size,
        );
    }

    /// Draws exits from adjacent rooms that face toward this room (only in Exits mode).
    fn draw_adjacent_exits(&self, ctx: &mut WgpuContext, grid_size: f32) {
        if !matches!(self.mode, TilemapEditorMode::Exits) {
            return;
        }

        for (world_grid_pos, direction) in &self.adjacent_exits {
            let world_pixel_pos = *world_grid_pos * grid_size;
            draw_adjacent_exit_arrow(ctx, world_pixel_pos, *direction, grid_size);
        }
    }

    fn draw_hover_highlight(
        &self,
        ctx: &mut WgpuContext,
        camera: &Camera2D,
        map: &TileMap,
        room_position: Vec2,
        grid_size: f32,
    ) {
        if self.is_mouse_over_ui(ctx, camera) {
            return;
        }

        let tile_pos = match self.mode {
            TilemapEditorMode::Tiles => {
                self.get_hovered_tile(ctx, camera, map, room_position, grid_size)
            }
            TilemapEditorMode::Exits => {
                self.get_hovered_edge(ctx, camera, map, room_position, grid_size)
            }
        };

        if let Some(tile_pos) = tile_pos {
            let line_width = thickness(grid_size);

            let x = tile_pos.x() as f32 * grid_size + room_position.x;
            let y = tile_pos.y() as f32 * grid_size + room_position.y;

            match self.mode {
                TilemapEditorMode::Tiles => {
                    ctx.draw_rectangle_lines(x, y, grid_size, grid_size, line_width, Color::RED);
                }
                TilemapEditorMode::Exits => {
                    let exit_direction = self.exit_direction_from_position(tile_pos, map);
                    draw_exit_arrow(ctx, vec2(x, y), exit_direction, grid_size);
                }
            }
        }
    }

    fn draw_ui(
        &mut self,
        ctx: &mut WgpuContext,
        camera: &Camera2D,
        asset_manager: &mut AssetManager,
        tilemap: &mut TileMap,
        room_position: Vec2,
        room_size: Vec2,
        grid_size: f32,
    ) {
        // Draw resize handles and preview
        for (i, handle) in self.resize_handles.iter().enumerate() {
            let is_active = self.active_handle_index == Some(i);
            handle.draw(ctx, camera, is_active, self.preview_valid, grid_size);

            // Draw preview if this handle is being dragged
            if is_active {
                handle.draw_preview(ctx, room_position, room_size, grid_size, self.preview_valid);
            }
        }

        // Static UI cam
        ctx.set_default_camera();

        // Top menu background
        draw_top_panel_full(ctx);

        // Draw inspector panel
        self.tilemap_panel.draw(ctx, asset_manager, tilemap);
    }

    fn get_hovered_tile(
        &self,
        ctx: &WgpuContext,
        camera: &Camera2D,
        map: &TileMap,
        room_position: Vec2,
        grid_size: f32,
    ) -> Option<GridPos> {
        let mouse_pos: Vec2 = ctx.mouse_position().into();
        let world_pos = camera.screen_to_world(mouse_pos, ctx.screen_width(), ctx.screen_height());
        let local_pos = world_pos - room_position;
        let pos = GridPos::from_world(local_pos, grid_size);

        if pos.is_in_bounds(map.width, map.height) {
            Some(pos)
        } else {
            None
        }
    }

    fn get_hovered_edge(
        &self,
        ctx: &WgpuContext,
        camera: &Camera2D,
        map: &TileMap,
        room_position: Vec2,
        grid_size: f32,
    ) -> Option<GridPos> {
        let mouse_pos: Vec2 = ctx.mouse_position().into();
        let world_pos = camera.screen_to_world(mouse_pos, ctx.screen_width(), ctx.screen_height());
        let local_pos = world_pos - room_position;
        let edge_pos = GridPos::from_world_edge(local_pos, map, grid_size);

        let x_outside = edge_pos.x() < 0 || edge_pos.x() >= map.width as i32;
        let y_outside = edge_pos.y() < 0 || edge_pos.y() >= map.height as i32;

        // Only allow positions strictly outside one axis (no corners)
        if x_outside ^ y_outside {
            Some(edge_pos)
        } else {
            None
        }
    }

    fn is_mouse_over_ui(&self, ctx: &WgpuContext, camera: &Camera2D) -> bool {
        let mouse_screen: Vec2 = ctx.mouse_position().into();
        let mouse_world =
            camera.screen_to_world(mouse_screen, ctx.screen_width(), ctx.screen_height());

        // Check menu bar area
        let over_menu_bar = mouse_screen.y < MENU_PANEL_HEIGHT;

        // Check sub-mode strip
        let over_sub_mode = self.sub_mode_rect.is_some_and(|r| r.contains(mouse_screen));

        over_menu_bar
            || over_sub_mode
            || self.tilemap_panel.is_mouse_over(mouse_screen)
            || self
                .resize_handles
                .iter()
                .any(|h| h.is_hovered(mouse_world))
            || self.active_handle_index.is_some()
            || is_dropdown_open()
            || is_modal_open()
            || is_mouse_over_panel(ctx)
    }

    fn exit_direction_from_position(&self, tile_pos: GridPos, map: &TileMap) -> ExitDirection {
        match tile_pos {
            GridPos(p) if p.y == -1 => ExitDirection::Up,
            GridPos(p) if p.y == map.height as i32 => ExitDirection::Down,
            GridPos(p) if p.x == -1 => ExitDirection::Left,
            GridPos(p) if p.x == map.width as i32 => ExitDirection::Right,
            GridPos(p) if p.y == 0 => ExitDirection::Up,
            GridPos(p) if p.y as usize == map.height - 1 => ExitDirection::Down,
            GridPos(p) if p.x == 0 => ExitDirection::Left,
            GridPos(p) if p.x as usize == map.width - 1 => ExitDirection::Right,
            _ => ExitDirection::Up, // default for safety
        }
    }

    pub fn reset(&mut self) {
        self.mode = TilemapEditorMode::Tiles;
        self.initialized = false;
        self.ui_was_clicked = false;
        self.active_handle_index = None;
        self.resize_handles.clear();
        self.adjacent_exits.clear();
        self.sub_mode_rect = None;
    }

}

fn resize_result_message(result: ResizeResult) -> Option<&'static str> {
    match result {
        ResizeResult::InvalidDimensions => Some("Invalid resize dimensions"),
        ResizeResult::Overlap => Some("Resize can not overlap rooms"),
        ResizeResult::StrandedExit => Some("Resize can not strand exits"),
        ResizeResult::Success => None,
    }
}
