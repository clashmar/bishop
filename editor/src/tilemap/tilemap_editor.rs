use crate::commands::room::*;
use crate::editor_assets::assets::*;
use crate::editor_global::{push_command, push_toast};
use crate::gui::gui_constants::MENU_PANEL_HEIGHT;
use crate::gui::mode_selector::ModeInfo;
use crate::room::drawing::*;
use crate::shared::input::canvas_blocked_by_global_ui;
use crate::tilemap::resize_handle::*;
use bishop::prelude::*;
use engine_core::assets::SpriteManager;
use engine_core::controls::Controls;
use engine_core::ecs::{Ecs, TilePlacement};
use engine_core::tiles::{draw_room_tile_placements, TileDefId, TileMap, TileRegistry};
use engine_core::worlds::*;

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
    selected_tile_def: Option<TileDefId>,
    external_ui_blocked: bool,
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
            selected_tile_def: None,
            external_ui_blocked: false,
            ui_was_clicked: false,
            initialized: false,
            adjacent_exits: Vec::new(),
            sub_mode_rect: None,
        }
    }

    pub fn set_selected_tile(&mut self, selected_tile_def: Option<TileDefId>) {
        self.selected_tile_def = selected_tile_def;
    }

    pub fn sync_adjacent_exits(&mut self, adjacent_exits: &[(Vec2, ExitDirection)]) {
        self.adjacent_exits.clear();
        self.adjacent_exits.extend_from_slice(adjacent_exits);
    }

    pub fn update(
        &mut self,
        ctx: &WgpuContext,
        inspector_blocked: bool,
        camera: &Camera2D,
        room: &mut Room,
        ecs: &Ecs,
        other_bounds: &[(Vec2, Vec2)],
        grid_size: f32,
    ) {
        if !self.initialized {
            self.ui_was_clicked = true;
            self.initialized = true;
        }

        self.external_ui_blocked = inspector_blocked;

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

        let drag_active =
            self.handle_resize_drag(ctx, mouse_world, room, other_bounds, grid_size, room.id);

        self.consume_ui_click(ctx, camera);

        if !self.ui_was_clicked && !drag_active {
            let room_position = room.position;
            let idx = room.current_variant_index();
            match self.mode {
                TilemapEditorMode::Tiles => self.handle_tile_placement(
                    ctx,
                    camera,
                    &room.variants[idx].tilemap,
                    room.id,
                    ecs,
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

    pub fn draw(
        &mut self,
        ctx: &mut WgpuContext,
        camera: &Camera2D,
        room: &mut Room,
        assets: (&TileRegistry, &mut SpriteManager),
        ecs: &Ecs,
        grid_size: f32,
    ) {
        let (tile_registry, sprite_manager) = assets;
        let variant_index = room.current_variant_index();
        let tilemap = &room.variants[variant_index].tilemap;
        let room_position = room.position;
        let room_id = room.id;
        let room_size = room.size;

        ctx.clear_background(Color::BLACK);
        ctx.set_camera(camera);
        tilemap.draw_background(ctx, room_position, grid_size);
        draw_room_tile_placements(
            ctx,
            ecs,
            room_id,
            tile_registry,
            sprite_manager,
            room_position,
            grid_size,
        );
        draw_exit_placeholders(ctx, &room.exits, room_position, grid_size);
        self.draw_adjacent_exits(ctx, grid_size);
        self.draw_hover_highlight(ctx, camera, tilemap, room_position, grid_size);

        if self.active_handle_index.is_some() {
            draw_all_camera_viewports(ctx, camera, ecs, room_id);
        }

        self.draw_resize_handles(
            ctx,
            camera,
            Rect::new(room_position.x, room_position.y, room_size.x, room_size.y),
            grid_size,
        );
    }

    pub fn reset(&mut self) {
        self.mode = TilemapEditorMode::Tiles;
        self.initialized = false;
        self.selected_tile_def = None;
        self.external_ui_blocked = false;
        self.ui_was_clicked = false;
        self.active_handle_index = None;
        self.resize_handles.clear();
        self.adjacent_exits.clear();
        self.sub_mode_rect = None;
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

    fn consume_ui_click(&mut self, ctx: &WgpuContext, camera: &Camera2D) {
        if (ctx.is_mouse_button_pressed(MouseButton::Left)
            || ctx.is_mouse_button_pressed(MouseButton::Right))
            && self.is_mouse_over_ui(ctx, camera)
        {
            self.ui_was_clicked = true;
            return;
        }

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
        map: &TileMap,
        room_id: RoomId,
        ecs: &Ecs,
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

        let existing = ecs
            .entities_in_room(room_id)
            .iter()
            .copied()
            .filter_map(|entity| ecs.get::<TilePlacement>(entity).copied())
            .find(|tile| (tile.grid_x, tile.grid_y) == (x, y));

        if ctx.is_mouse_button_down(MouseButton::Left) && ctx.is_key_down(KeyCode::LeftAlt) {
            if existing.is_some() {
                push_command(Box::new(SetTilePlacementCmd::clear(room_id, (x, y))));
            }
            return;
        }

        let Some(def_id) = self.selected_tile_def else {
            return;
        };

        if ctx.is_mouse_button_down(MouseButton::Left)
            && existing.is_none_or(|tile| tile.definition != def_id)
        {
            push_command(Box::new(SetTilePlacementCmd::place(room_id, (x, y), def_id)));
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

    fn draw_resize_handles(
        &mut self,
        ctx: &mut WgpuContext,
        camera: &Camera2D,
        room_rect: Rect,
        grid_size: f32,
    ) {
        for (i, handle) in self.resize_handles.iter().enumerate() {
            let is_active = self.active_handle_index == Some(i);
            handle.draw(ctx, camera, is_active, self.preview_valid, grid_size);

            if is_active {
                handle.draw_preview(
                    ctx,
                    room_rect.top_left(),
                    vec2(room_rect.w, room_rect.h),
                    grid_size,
                    self.preview_valid,
                );
            }
        }
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
            || self.external_ui_blocked
            || self
                .resize_handles
                .iter()
                .any(|h| h.is_hovered(mouse_world))
            || self.active_handle_index.is_some()
            || canvas_blocked_by_global_ui(ctx)
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

}

fn resize_result_message(result: ResizeResult) -> Option<&'static str> {
    match result {
        ResizeResult::InvalidDimensions => Some("Invalid resize dimensions"),
        ResizeResult::Overlap => Some("Resize can not overlap rooms"),
        ResizeResult::StrandedExit => Some("Resize can not strand exits"),
        ResizeResult::Success => None,
    }
}
