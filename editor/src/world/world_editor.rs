use std::collections::HashMap;

use crate::app::EditorCameraController;
use crate::app::SubEditor;
use crate::canvas::grid;
use crate::canvas::grid_shader::GridRenderer;
use crate::editor_assets::assets::*;
use crate::gui::inspector::shell::Inspector;
use crate::gui::mode_selector::*;
use crate::shared::input::{canvas_blocked_by_global_ui, shortcuts_blocked};
use crate::world::coord::*;
use crate::world::drawing::{collect_nav_icons, draw_navigation_icons, scaled_room_rect};
use bishop::prelude::*;
use engine_core::controls::Controls;
use engine_core::game::Game;
use engine_core::worlds::*;
use once_cell::sync::Lazy;
use strum::IntoEnumIterator;
use strum_macros::EnumIter;

pub const LINE_THICKNESS_MULTIPLIER: f32 = 0.005;
pub(super) const HOVER_LINE_THICKNESS: f32 = 0.01;

#[derive(Clone, Copy, PartialEq, EnumIter)]
pub enum WorldEditorMode {
    Select,
    New,
    Delete,
}

impl ModeInfo for WorldEditorMode {
    fn label(&self) -> &'static str {
        match self {
            WorldEditorMode::Select => "Select: S",
            WorldEditorMode::New => "New Room: N",
            WorldEditorMode::Delete => "Delete Room: D",
        }
    }
    fn icon(&self) -> &'static Texture2D {
        match self {
            WorldEditorMode::Select => select_icon(),
            WorldEditorMode::New => create_icon(),
            WorldEditorMode::Delete => delete_icon(),
        }
    }
    fn shortcut(self) -> Option<fn(&WgpuContext) -> bool> {
        match self {
            WorldEditorMode::Select => Some(Controls::s),
            WorldEditorMode::New => Some(Controls::n),
            WorldEditorMode::Delete => Some(Controls::d),
        }
    }
}

pub struct WorldEditor {
    pub(super) mode: WorldEditorMode,
    pub(super) mode_selector: ModeSelector<WorldEditorMode>,
    pub(super) active_rects: Vec<Rect>,
    pub(super) inspector: Inspector,
    show_grid: bool,
    pub(super) placing_start: Option<Vec2>,
    pub(super) placing_end: Option<Vec2>,
    pub pending_camera_focus: Option<Vec2>,
}

impl WorldEditor {
    pub fn new() -> Self {
        let active_rects: Vec<Rect> = Vec::new();
        let mode = WorldEditorMode::Select;
        let inspector = Inspector::new();

        Self {
            mode,
            mode_selector: ModeSelector {
                current: mode,
                options: *ALL_MODES,
            },
            active_rects,
            inspector,
            show_grid: true,
            placing_start: None,
            placing_end: None,
            pending_camera_focus: None,
        }
    }

    /// Returns `Some(room_id)` if a room is clicked on.
    pub fn update(
        &mut self,
        ctx: &mut WgpuContext,
        camera: &mut Camera2D,
        game: &mut Game,
    ) -> Option<RoomId> {
        if let Some(world) = game.current_world_mut() {
            world.link_all_room_exits();
        }

        self.handle_mouse_cursor(ctx);
        self.handle_shortcuts(ctx);

        match self.mode {
            WorldEditorMode::Select => {
                let world = game
                    .current_world_mut()
                    .expect("World editor requires a world");
                self.update_selecting_mode(ctx, camera, world)
            }
            WorldEditorMode::New => self.update_placing_mode(ctx, camera, game),
            WorldEditorMode::Delete => self.update_deleting_mode(ctx, camera, game),
        }
    }

    fn update_selecting_mode(
        &mut self,
        ctx: &WgpuContext,
        camera: &Camera2D,
        world: &mut World,
    ) -> Option<RoomId> {
        if ctx.is_mouse_button_pressed(MouseButton::Left) && !self.should_block_canvas(ctx) {
            let world_mouse = mouse_world_pos(ctx, camera);
            for room in world.rooms() {
                let rect = scaled_room_rect(room, world.grid_size);
                if rect.contains(world_mouse) {
                    return Some(room.id);
                }
            }
        }
        None
    }

    fn update_deleting_mode(
        &mut self,
        ctx: &WgpuContext,
        camera: &Camera2D,
        game: &mut Game,
    ) -> Option<RoomId> {
        if ctx.is_mouse_button_pressed(MouseButton::Left) && !self.should_block_canvas(ctx) {
            let world_mouse = mouse_world_pos(ctx, camera);
            let cur_world = game.current_world();
            for room in cur_world.rooms() {
                let rect = scaled_room_rect(room, cur_world.grid_size);
                if rect.contains(world_mouse) {
                    let room_id = room.id;
                    let mut game_ctx = game.ctx_mut();
                    self.delete_room(&mut game_ctx, room_id);
                    return None;
                }
            }
        }
        None
    }

    fn update_placing_mode(
        &mut self,
        ctx: &WgpuContext,
        camera: &Camera2D,
        game: &mut Game,
    ) -> Option<RoomId> {
        if self.should_block_canvas(ctx) {
            return None;
        }

        let grid_size = game.current_world().grid_size;
        let mouse_tile = snap_to_grid(mouse_world_grid(ctx, camera, grid_size));

        if ctx.is_mouse_button_pressed(MouseButton::Left) {
            self.placing_start = Some(mouse_tile);
            self.placing_end = Some(mouse_tile);
        }

        if ctx.is_mouse_button_down(MouseButton::Left) {
            self.placing_end = Some(mouse_tile);
        }

        if ctx.is_mouse_button_released(MouseButton::Left) {
            if let (Some(start), Some(end)) = (self.placing_start, self.placing_end) {
                let (top_left, size) = rect_from_points(start, end);
                let rooms = game.current_world().rooms();
                let should_create =
                    !self.intersects_existing_room(rooms, top_left, size, grid_size);

                if should_create {
                    // Create the room and get its id back.
                    let new_id = self.place_room_from_drag(game, top_left, size, grid_size);
                    self.reset_placing();
                    self.reset();
                    return Some(new_id);
                }
                // Overlap – just abort placement.
                self.reset_placing();
            }
        }
        None
    }

    pub(super) fn intersects_existing_room(
        &self,
        rooms: &[Room],
        top_left: Vec2,
        size: Vec2,
        grid_size: f32,
    ) -> bool {
        let bounds: Vec<(Vec2, Vec2)> = rooms.iter().map(|rm| (rm.position, rm.size)).collect();

        overlaps_existing_rooms(top_left, size, &bounds, grid_size)
    }

    fn reset_placing(&mut self) {
        self.placing_start = None;
        self.placing_end = None;
    }

    pub fn draw(
        &mut self,
        ctx: &mut WgpuContext,
        world_id: WorldId,
        camera: &Camera2D,
        game: &mut Game,
        grid_renderer: &GridRenderer,
    ) {
        ctx.set_camera(camera);
        ctx.clear_background(Color::LIGHTGREY);

        let nav_icons_visible = self.inspector.world.show_navigation_icons();

        let (grid_size, room_map) = {
            let world = game
                .get_world_mut(world_id)
                .expect("World editor requires a world");
            let grid_size = world.grid_size;
            let room_map: HashMap<RoomId, Vec2> = world.rooms()
                .iter()
                .map(|r| (r.id, r.position))
                .collect();
            (grid_size, room_map)
        };

        let nav_icons = collect_nav_icons(&game.ecs, &room_map, grid_size);

        let world = game
            .get_world_mut(world_id)
            .expect("World editor requires a world");

        let rooms = world.rooms();

        grid::draw_grid(ctx, grid_renderer, camera, grid_size);

        self.draw_rooms(ctx, camera, rooms, grid_size);
        self.draw_exits(ctx, rooms, grid_size);

        if !self.should_block_canvas(ctx) {
            match self.mode {
                WorldEditorMode::Select => {
                    self.draw_hovered_room(ctx, camera, rooms, grid_size);
                }
                WorldEditorMode::Delete => {
                    self.draw_hovered_room(ctx, camera, rooms, grid_size);
                }
                WorldEditorMode::New => {
                    self.draw_placing_preview(ctx, camera, rooms, grid_size);
                }
            }
        }

        if nav_icons_visible {
            draw_navigation_icons(ctx, &nav_icons, grid_size);
        }

        self.draw_room_names(ctx, camera, rooms, grid_size);
        ctx.set_default_camera();
        self.draw_coordinates(ctx, camera, grid_size);

        self.draw_ui(ctx, camera, game, world_id);
    }

    pub fn init_camera(&mut self, ctx: &WgpuContext, camera: &mut Camera2D, world: &World) {
        let target_room = world.rooms().first();

        if let Some(room) = target_room {
            self.center_on_room(ctx, camera, room, world.grid_size);
        }
    }

    /// Centres the camera on `room` without changing zoom.
    pub fn center_on_room(
        &mut self,
        ctx: &WgpuContext,
        camera: &mut Camera2D,
        room: &Room,
        grid_size: f32,
    ) {
        *camera = EditorCameraController::camera_for_room(ctx, room.size, room.position, grid_size);
    }

    /// Centres the camera on `pos` without changing zoom.
    pub fn center_on_position(&self, camera: &mut Camera2D, pos: Vec2) {
        camera.target = pos;
    }

    fn handle_shortcuts(&mut self, ctx: &WgpuContext) {
        if Controls::g(ctx) {
            self.show_grid = !self.show_grid;
        }

        for mode in WorldEditorMode::iter() {
            if let Some(shortcut) = mode.shortcut() {
                if shortcut(ctx) && !shortcuts_blocked() {
                    self.mode = mode;
                    self.mode_selector.current = mode;
                    break;
                }
            }
        }
    }

    #[inline]
    pub(super) fn register_rect(&mut self, rect: Rect) -> Rect {
        self.active_rects.push(rect);
        rect
    }

    fn handle_mouse_cursor(&self, ctx: &mut WgpuContext) {
        if self.should_block_canvas(ctx) {
            ctx.set_cursor_icon(CursorIcon::Default);
        } else {
            match self.mode {
                WorldEditorMode::Select => {
                    ctx.set_cursor_icon(CursorIcon::Pointer);
                }
                WorldEditorMode::New => {
                    ctx.set_cursor_icon(CursorIcon::Crosshair);
                }
                WorldEditorMode::Delete => {
                    ctx.set_cursor_icon(CursorIcon::Crosshair);
                }
            }
        }
    }

    pub fn reset(&mut self) {
        self.mode = WorldEditorMode::Select;
        self.mode_selector.current = WorldEditorMode::Select;
        self.placing_start = None;
        self.placing_end = None;
        self.active_rects.clear();
        self.show_grid = true;
    }
}

impl SubEditor for WorldEditor {
    fn active_rects(&self) -> &[Rect] {
        &self.active_rects
    }

    fn should_block_canvas(&self, ctx: &WgpuContext) -> bool {
        let mouse_screen: Vec2 = ctx.mouse_position().into();
        self.active_rects.iter().any(|r| r.contains(mouse_screen))
            || self.inspector.is_mouse_over(ctx)
            || canvas_blocked_by_global_ui(ctx)
    }
}

/// A slice of all the modes.
static ALL_MODES: Lazy<&'static [WorldEditorMode]> =
    Lazy::new(|| Box::leak(Box::new(WorldEditorMode::iter().collect::<Vec<_>>())));
