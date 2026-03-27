// editor/src/world/world_editor.rs
use crate::app::EditorCameraController;
use crate::app::SubEditor;
use crate::canvas::grid;
use crate::canvas::grid_shader::GridRenderer;
use crate::editor_assets::assets::*;
use crate::gui::menu_bar::*;
use crate::gui::mode_selector::*;
use crate::world::coord::*;
use bishop::prelude::*;
use engine_core::prelude::*;
use once_cell::sync::Lazy;
use strum::IntoEnumIterator;
use strum_macros::EnumIter;

pub const LINE_THICKNESS_MULTIPLIER: f32 = 0.005;
const HIGHLIGHT_COLOR: Color = Color::new(0.0, 1.0, 0.0, 0.5);
const HIGHLIGHT_ERROR_COLOR: Color = Color::new(1.0, 0.0, 0.0, 0.5);
const ROOM_LINE_INSET: f32 = 1.0;
const HOVER_LINE_THICKNESS: f32 = 0.01;

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
    mode: WorldEditorMode,
    mode_selector: ModeSelector<WorldEditorMode>,
    active_rects: Vec<Rect>,
    show_grid: bool,
    placing_start: Option<Vec2>,
    placing_end: Option<Vec2>,
}

impl WorldEditor {
    pub fn new() -> Self {
        let active_rects: Vec<Rect> = Vec::new();
        let mode = WorldEditorMode::Select;

        Self {
            mode,
            mode_selector: ModeSelector {
                current: mode,
                options: *ALL_MODES,
            },
            active_rects,
            show_grid: true,
            placing_start: None,
            placing_end: None,
        }
    }

    /// Returns `Some(room_id)` if a room is clicked on.
    pub async fn update(
        &mut self,
        ctx: &mut WgpuContext,
        camera: &mut Camera2D,
        game: &mut Game,
    ) -> Option<RoomId> {
        game.current_world_mut().link_all_exits();

        self.handle_mouse_cursor(ctx);
        self.handle_shortcuts(ctx);

        match self.mode {
            WorldEditorMode::Select => {
                self.update_selecting_mode(ctx, camera, game.current_world_mut())
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
            for room in &world.rooms {
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
            for room in &cur_world.rooms {
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
                let rooms = &game.current_world().rooms;
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

    fn intersects_existing_room(
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

        let world = game.get_world_mut(world_id);
        let rooms = &world.rooms;

        grid::draw_grid(ctx, grid_renderer, camera, world.grid_size);

        self.draw_rooms(ctx, camera, rooms, world.grid_size);
        self.draw_exits(ctx, rooms, world.grid_size);

        if !self.should_block_canvas(ctx) {
            match self.mode {
                WorldEditorMode::Select => {
                    self.draw_hovered_room(ctx, camera, rooms, world.grid_size);
                }
                WorldEditorMode::Delete => {
                    self.draw_hovered_room(ctx, camera, rooms, world.grid_size);
                }
                WorldEditorMode::New => {
                    self.draw_placing_preview(ctx, camera, rooms, world.grid_size);
                }
            }
        }

        self.draw_room_names(ctx, camera, rooms, world.grid_size);
        self.draw_ui(ctx, camera);

        // Static UI camera
        ctx.set_default_camera();
        self.draw_coordinates(ctx, camera, world.grid_size);
    }

    pub fn draw_rooms(
        &self,
        ctx: &mut WgpuContext,
        camera: &Camera2D,
        rooms: &Vec<Room>,
        grid_size: f32,
    ) {
        for room in rooms {
            let rect = scaled_room_rect(room, grid_size);
            let inset = ROOM_LINE_INSET * grid_size;

            // Draw the room outline
            ctx.draw_rectangle_lines(
                rect.x + inset / 2.0,
                rect.y + inset / 2.0,
                rect.w - inset,
                rect.h - inset,
                LINE_THICKNESS_MULTIPLIER / camera.zoom.x,
                Color::BLUE,
            );
        }
    }

    fn draw_exits(&self, ctx: &mut WgpuContext, rooms: &Vec<Room>, grid_size: f32) {
        for room in rooms {
            for exit in &room.exits {
                let exit_world_coord = (room.position / grid_size) + exit.position;
                // Decide color based on whether it's linked
                let color = if exit.target_room_id.is_some() {
                    Color::GREEN
                } else {
                    Color::RED
                };
                self.draw_exit_marker(ctx, exit_world_coord, exit.direction, color, grid_size);
            }
        }
    }

    fn draw_exit_marker(
        &self,
        ctx: &mut WgpuContext,
        exit_world_coord: Vec2,
        dir: ExitDirection,
        color: Color,
        grid_size: f32,
    ) {
        const THICKNESS: f32 = 2.0;
        let length = grid_size;
        let offset = 1.0;

        match dir {
            ExitDirection::Up => ctx.draw_rectangle(
                exit_world_coord.x * grid_size,
                exit_world_coord.y * grid_size + grid_size,
                length,
                THICKNESS,
                color,
            ),
            ExitDirection::Down => ctx.draw_rectangle(
                exit_world_coord.x * grid_size,
                exit_world_coord.y * grid_size - THICKNESS + offset,
                length,
                THICKNESS,
                color,
            ),
            ExitDirection::Left => ctx.draw_rectangle(
                (exit_world_coord.x + 1.0) * grid_size - offset,
                exit_world_coord.y * grid_size,
                THICKNESS,
                length,
                color,
            ),
            ExitDirection::Right => ctx.draw_rectangle(
                (exit_world_coord.x - 1.0) * grid_size + grid_size - THICKNESS + offset,
                exit_world_coord.y * grid_size,
                THICKNESS,
                length,
                color,
            ),
        }
    }

    fn draw_hovered_room(
        &self,
        ctx: &mut WgpuContext,
        camera: &Camera2D,
        rooms: &Vec<Room>,
        grid_size: f32,
    ) {
        let world_mouse = mouse_world_pos(ctx, camera);
        for room in rooms {
            let rect = scaled_room_rect(room, grid_size);
            if rect.contains(world_mouse) {
                let inset = ROOM_LINE_INSET * grid_size;

                // Choose highlight color based on mode
                let color = match self.mode {
                    WorldEditorMode::Delete => HIGHLIGHT_ERROR_COLOR,
                    _ => HIGHLIGHT_COLOR,
                };

                ctx.draw_rectangle(
                    rect.x + inset / 2.0,
                    rect.y + inset / 2.0,
                    rect.w - inset,
                    rect.h - inset,
                    color,
                );

                break; // only highlight one room
            }
        }
    }

    fn draw_room_names(
        &self,
        ctx: &mut WgpuContext,
        camera: &Camera2D,
        rooms: &Vec<Room>,
        grid_size: f32,
    ) {
        ctx.set_default_camera(); // draw in screen space

        for room in rooms {
            let rect = scaled_room_rect(room, grid_size);

            // Screen coordinates of room center
            let screen_pos = camera.world_to_screen(
                rect.top_left() + rect.size() / 2.0,
                ctx.screen_width(),
                ctx.screen_height(),
            );

            // Base text size
            let base_font_size: f32 = 40.0;

            // Scale based on room size and camera zoom
            let room_scale = (rect.w + rect.h) / 2.0 / 60.0;
            let zoom_factor = camera.zoom.x * 100.0;
            let font_size = (base_font_size * room_scale * zoom_factor).clamp(10.0, 200.0);

            // Rotation: vertical if tall
            let rotation = if rect.h > rect.w {
                std::f32::consts::FRAC_PI_2
            } else {
                0.0
            };

            // Measure text to center it properly
            let dims = ctx.measure_text(&room.name, font_size);

            // Center text at room center (x - half_width, y + ascent - half_height)
            let x = screen_pos.x - dims.width / 2.0;
            let y = screen_pos.y + dims.offset_y - dims.height / 2.0;

            ctx.draw_text_ex(
                &room.name,
                x,
                y,
                TextParams {
                    font_size: font_size as u16,
                    color: Color::BLACK,
                    rotation,
                    ..Default::default()
                },
            );
        }

        ctx.set_camera(camera); // back to world camera
    }

    fn draw_placing_preview(
        &self,
        ctx: &mut WgpuContext,
        camera: &Camera2D,
        rooms: &[Room],
        grid_size: f32,
    ) {
        if let (Some(start), Some(end)) = (self.placing_start, self.placing_end) {
            let (top_left, size) = rect_from_points(start, end);
            let color = if self.intersects_existing_room(rooms, top_left, size, grid_size) {
                HIGHLIGHT_ERROR_COLOR
            } else {
                HIGHLIGHT_COLOR
            };
            let inset = ROOM_LINE_INSET * grid_size;
            ctx.draw_rectangle_lines(
                top_left.x * grid_size + inset / 2.0,
                top_left.y * grid_size + inset / 2.0,
                size.x * grid_size - inset,
                size.y * grid_size - inset,
                HOVER_LINE_THICKNESS / camera.zoom.x,
                color,
            );
        } else {
            let hover_tile = snap_to_grid(mouse_world_grid(ctx, camera, grid_size));
            let color =
                if self.intersects_existing_room(rooms, hover_tile, vec2(1.0, 1.0), grid_size) {
                    HIGHLIGHT_ERROR_COLOR
                } else {
                    HIGHLIGHT_COLOR
                };
            ctx.draw_rectangle(
                hover_tile.x * grid_size,
                hover_tile.y * grid_size,
                grid_size,
                grid_size,
                color,
            );
        }
    }

    fn draw_ui(&mut self, ctx: &mut WgpuContext, camera: &Camera2D) {
        self.active_rects.clear();

        // Static camera
        ctx.set_default_camera();

        // Top menu panel
        self.register_rect(draw_top_panel_full(ctx));

        // Mode selector
        if self.mode_selector.draw(ctx).1 {
            self.mode = self.mode_selector.current;
        }
        self.mode_selector.draw_tooltips(ctx);

        ctx.set_camera(camera); // Back to world camera
    }

    pub fn init_camera(&mut self, ctx: &WgpuContext, camera: &mut Camera2D, world: &World) {
        let target_room = world
            .starting_room_id
            .and_then(|id| world.get_room(id))
            .or_else(|| world.rooms.first());

        if let Some(room) = target_room {
            self.center_on_room(ctx, camera, room, world.grid_size);
        }
    }

    pub fn center_on_room(
        &mut self,
        ctx: &WgpuContext,
        camera: &mut Camera2D,
        room: &Room,
        grid_size: f32,
    ) {
        *camera = EditorCameraController::camera_for_room(ctx, room.size, room.position, grid_size);
    }

    fn handle_shortcuts(&mut self, ctx: &WgpuContext) {
        if Controls::g(ctx) {
            self.show_grid = !self.show_grid;
        }

        for mode in WorldEditorMode::iter() {
            if let Some(shortcut) = mode.shortcut() {
                if shortcut(ctx) && !input_is_focused() {
                    self.mode = mode;
                    self.mode_selector.current = mode;
                    break;
                }
            }
        }
    }

    #[inline]
    fn register_rect(&mut self, rect: Rect) -> Rect {
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
}

/// Returns rect scaled for drawing
fn scaled_room_rect(room: &Room, grid_size: f32) -> Rect {
    let size = room.size;
    Rect::new(
        room.position.x,
        room.position.y,
        size.x * grid_size,
        size.y * grid_size,
    )
}

/// Compute top-left and size from any two points
fn rect_from_points(p1: Vec2, p2: Vec2) -> (Vec2, Vec2) {
    let top_left = vec2(p1.x.min(p2.x), p1.y.min(p2.y));
    let size = vec2(
        (p1.x - p2.x).abs().floor() + 1.0,
        (p1.y - p2.y).abs().floor() + 1.0,
    );
    (top_left, size)
}

/// A slice of all the modes.
static ALL_MODES: Lazy<&'static [WorldEditorMode]> =
    Lazy::new(|| Box::leak(Box::new(WorldEditorMode::iter().collect::<Vec<_>>())));
