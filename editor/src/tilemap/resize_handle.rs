// editor/src/gui/resize_handle.rs
use crate::world::coord::overlaps_existing_rooms;
use bishop::prelude::*;
use engine_core::prelude::*;

/// Which side of the tilemap the handle controls.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HandleSide {
    Top,
    Bottom,
    Left,
    Right,
}

/// Tracks the state of an active drag operation.
#[derive(Clone, Debug, Default)]
pub struct DragState {
    pub is_dragging: bool,
    pub start_mouse_world: Vec2,
    pub preview_delta: i32,
}

/// A draggable handle for resizing the tilemap from one side.
#[derive(Clone, Debug)]
pub struct ResizeHandle {
    pub side: HandleSide,
    pub rect: Rect,
    pub drag_state: DragState,
}

/// Result of tilemap resize.
#[derive(Debug, PartialEq, Eq)]
pub enum ResizeResult {
    Success,
    InvalidDimensions,
    StrandedExit,
    Overlap,
}

/// Data needed for resize preview.
pub(crate) struct PreviewData {
    position: Vec2,
    size: Vec2,
}

impl ResizeHandle {
    /// Creates a new handle for the given side.
    pub fn new(side: HandleSide, rect: Rect) -> Self {
        Self {
            side,
            rect,
            drag_state: DragState::default(),
        }
    }

    /// Build all 4 resize handles positioned around the tilemap.
    pub fn build_all(map: &TileMap, room_position: Vec2, grid_size: f32) -> Vec<ResizeHandle> {
        let map_pixel_width = map.width as f32 * grid_size;
        let map_pixel_height = map.height as f32 * grid_size;

        let thickness = grid_size / 2.0;
        let length = grid_size * 2.0;
        let offset = grid_size * 1.5;

        let handles = vec![
            ResizeHandle::new(
                HandleSide::Top,
                Rect::new(
                    room_position.x + map_pixel_width / 2.0 - length / 2.0,
                    room_position.y - offset - thickness / 2.0,
                    length,
                    thickness,
                ),
            ),
            ResizeHandle::new(
                HandleSide::Bottom,
                Rect::new(
                    room_position.x + map_pixel_width / 2.0 - length / 2.0,
                    room_position.y + map_pixel_height + offset - thickness / 2.0,
                    length,
                    thickness,
                ),
            ),
            ResizeHandle::new(
                HandleSide::Left,
                Rect::new(
                    room_position.x - offset - thickness / 2.0,
                    room_position.y + map_pixel_height / 2.0 - length / 2.0,
                    thickness,
                    length,
                ),
            ),
            ResizeHandle::new(
                HandleSide::Right,
                Rect::new(
                    room_position.x + map_pixel_width + offset - thickness / 2.0,
                    room_position.y + map_pixel_height / 2.0 - length / 2.0,
                    thickness,
                    length,
                ),
            ),
        ];

        handles
    }

    /// Check if the mouse is over this handle.
    pub fn is_hovered(&self, mouse_world: Vec2) -> bool {
        self.rect.contains(mouse_world)
    }

    /// Begin a drag operation, capturing the initial state.
    pub fn begin_drag(&mut self, mouse_world: Vec2) {
        self.drag_state = DragState {
            is_dragging: true,
            start_mouse_world: mouse_world,
            preview_delta: 0,
        };
    }

    /// Update the drag, computing the preview delta based on mouse movement.
    pub fn update_drag(&mut self, mouse_world: Vec2, grid_size: f32) -> i32 {
        if !self.drag_state.is_dragging {
            return 0;
        }

        let mouse_delta = mouse_world - self.drag_state.start_mouse_world;

        let delta = match self.side {
            HandleSide::Top => (-mouse_delta.y / grid_size).round() as i32,
            HandleSide::Bottom => (mouse_delta.y / grid_size).round() as i32,
            HandleSide::Left => (-mouse_delta.x / grid_size).round() as i32,
            HandleSide::Right => (mouse_delta.x / grid_size).round() as i32,
        };

        self.drag_state.preview_delta = delta;
        delta
    }

    /// Compute the preview bounds after applying the delta.
    pub fn compute_preview_bounds(
        &self,
        room_position: Vec2,
        room_size: Vec2,
        grid_size: f32,
    ) -> PreviewData {
        let delta = self.drag_state.preview_delta;
        let delta_pixels = delta as f32 * grid_size;

        let (new_pos, new_size) = match self.side {
            HandleSide::Top => (
                vec2(room_position.x, room_position.y - delta_pixels),
                vec2(room_size.x, room_size.y + delta as f32),
            ),
            HandleSide::Bottom => (room_position, vec2(room_size.x, room_size.y + delta as f32)),
            HandleSide::Left => (
                vec2(room_position.x - delta_pixels, room_position.y),
                vec2(room_size.x + delta as f32, room_size.y),
            ),
            HandleSide::Right => (room_position, vec2(room_size.x + delta as f32, room_size.y)),
        };

        PreviewData {
            position: new_pos,
            size: new_size * grid_size,
        }
    }

    /// End the drag operation.
    pub fn end_drag(&mut self) {
        self.drag_state.is_dragging = false;
    }

    /// Compute the visual position of the handle during a drag.
    pub fn current_draw_rect(&self, grid_size: f32) -> Rect {
        if !self.drag_state.is_dragging {
            return self.rect;
        }

        let delta_pixels = self.drag_state.preview_delta as f32 * grid_size;

        match self.side {
            HandleSide::Top => Rect::new(
                self.rect.x,
                self.rect.y - delta_pixels,
                self.rect.w,
                self.rect.h,
            ),
            HandleSide::Bottom => Rect::new(
                self.rect.x,
                self.rect.y + delta_pixels,
                self.rect.w,
                self.rect.h,
            ),
            HandleSide::Left => Rect::new(
                self.rect.x - delta_pixels,
                self.rect.y,
                self.rect.w,
                self.rect.h,
            ),
            HandleSide::Right => Rect::new(
                self.rect.x + delta_pixels,
                self.rect.y,
                self.rect.w,
                self.rect.h,
            ),
        }
    }

    /// Draw the handle rectangle.
    pub fn draw(
        &self,
        ctx: &mut WgpuContext,
        camera: &Camera2D,
        is_active: bool,
        preview_valid: bool,
        grid_size: f32,
    ) {
        let draw_rect = self.current_draw_rect(grid_size);

        let color = if is_active {
            if preview_valid {
                Color::new(0.0, 1.0, 0.0, 0.7) // Green for valid
            } else {
                Color::new(1.0, 0.0, 0.0, 0.7) // Red for invalid
            }
        } else {
            Color::new(0.3, 0.5, 1.0, 0.5) // Blue for idle
        };

        ctx.draw_rectangle(draw_rect.x, draw_rect.y, draw_rect.w, draw_rect.h, color);

        // Draw border for visibility
        let zoom_scale = camera.zoom.x.abs();
        let line_width = (0.5 / zoom_scale).clamp(1.0, 3.0);
        ctx.draw_rectangle_lines(
            draw_rect.x,
            draw_rect.y,
            draw_rect.w,
            draw_rect.h,
            line_width,
            Color::WHITE,
        );
    }

    /// Draw the preview overlay showing the proposed new bounds.
    pub fn draw_preview(
        &self,
        ctx: &mut WgpuContext,
        room_position: Vec2,
        room_size: Vec2,
        grid_size: f32,
        is_valid: bool,
    ) {
        if !self.drag_state.is_dragging || self.drag_state.preview_delta == 0 {
            return;
        }

        let delta = self.drag_state.preview_delta;
        let preview_data = self.compute_preview_bounds(room_position, room_size, grid_size);
        let preview_pos = preview_data.position;
        let preview_size = preview_data.size;

        // Calculate new dimensions in tiles
        let (new_width, new_height) = match self.side {
            HandleSide::Top | HandleSide::Bottom => {
                (room_size.x as i32, room_size.y as i32 + delta)
            }
            HandleSide::Left | HandleSide::Right => {
                (room_size.x as i32 + delta, room_size.y as i32)
            }
        };

        let color = if is_valid {
            Color::new(0.0, 1.0, 0.0, 0.2)
        } else {
            Color::new(1.0, 0.0, 0.0, 0.2)
        };

        ctx.draw_rectangle(
            preview_pos.x,
            preview_pos.y,
            preview_size.x,
            preview_size.y,
            color,
        );

        let border_color = if is_valid {
            Color::new(0.0, 1.0, 0.0, 0.8)
        } else {
            Color::new(1.0, 0.0, 0.0, 0.8)
        };
        ctx.draw_rectangle_lines(
            preview_pos.x,
            preview_pos.y,
            preview_size.x,
            preview_size.y,
            1.0,
            border_color,
        );

        // Draw dimension text centered above preview
        let dim_text = format!("{} x {}", new_width, new_height);
        let font_size = grid_size.max(16.0);
        let text_x =
            preview_pos.x + preview_size.x / 2.0 - (dim_text.len() as f32 * font_size * 0.3);
        let text_y = preview_pos.y - font_size * 0.5;
        let text_color = if is_valid { Color::GREEN } else { Color::RED };
        ctx.draw_text(&dim_text, text_x, text_y, font_size, text_color);
    }
}

/// Validates a resize drag and returns a `ResizeResult`.
pub fn validate_resize(
    map: &TileMap,
    exits: &[Exit],
    side: HandleSide,
    delta: i32,
    other_bounds: &[(Vec2, Vec2)],
    preview_data: PreviewData,
    grid_size: f32,
) -> ResizeResult {
    let (new_w, new_h) = compute_new_dims(map, side, delta);

    if !size_valid(map.width, map.height, side, delta) {
        return ResizeResult::InvalidDimensions;
    }
    if !exits_valid(map, exits, side, delta, new_w, new_h) {
        return ResizeResult::StrandedExit;
    }
    if !overlap_valid(
        preview_data.position,
        preview_data.size,
        other_bounds,
        grid_size,
    ) {
        return ResizeResult::Overlap;
    }
    ResizeResult::Success
}

fn compute_new_dims(map: &TileMap, side: HandleSide, delta: i32) -> (usize, usize) {
    match side {
        HandleSide::Top | HandleSide::Bottom => (map.width, (map.height as i32 + delta) as usize),
        HandleSide::Left | HandleSide::Right => ((map.width as i32 + delta) as usize, map.height),
    }
}

fn size_valid(map_width: usize, map_height: usize, side: HandleSide, delta: i32) -> bool {
    match side {
        HandleSide::Top | HandleSide::Bottom => {
            let new_h = map_height as i32 + delta;
            new_h >= 1
        }
        HandleSide::Left | HandleSide::Right => {
            let new_w = map_width as i32 + delta;
            new_w >= 1
        }
    }
}

fn exits_valid(
    map: &TileMap,
    exits: &[Exit],
    side: HandleSide,
    delta: i32,
    new_w: usize,
    new_h: usize,
) -> bool {
    exits
        .iter()
        .all(|e| is_exit_valid_after_resize(e, side, delta, map.width, map.height, new_w, new_h))
}

fn overlap_valid(
    preview_pos: Vec2,
    preview_size: Vec2,
    other_bounds: &[(Vec2, Vec2)],
    grid_size: f32,
) -> bool {
    // Convert from pixels to tile coordinates for consistency with overlaps_existing_rooms
    let tile_pos = preview_pos / grid_size;
    let tile_size = preview_size / grid_size;
    !overlaps_existing_rooms(tile_pos, tile_size, other_bounds, grid_size)
}

fn get_exit_side(exit: &Exit, width: usize, height: usize) -> Option<HandleSide> {
    let x = exit.position.x as i32;
    let y = exit.position.y as i32;

    if y == -1 {
        Some(HandleSide::Top)
    } else if y == height as i32 {
        Some(HandleSide::Bottom)
    } else if x == -1 {
        Some(HandleSide::Left)
    } else if x == width as i32 {
        Some(HandleSide::Right)
    } else {
        None
    }
}

fn is_opposite_side(a: HandleSide, b: HandleSide) -> bool {
    matches!(
        (a, b),
        (HandleSide::Top, HandleSide::Bottom)
            | (HandleSide::Bottom, HandleSide::Top)
            | (HandleSide::Left, HandleSide::Right)
            | (HandleSide::Right, HandleSide::Left)
    )
}

fn is_exit_valid_after_resize(
    exit: &Exit,
    drag_side: HandleSide,
    delta: i32,
    old_width: usize,
    old_height: usize,
    new_width: usize,
    new_height: usize,
) -> bool {
    let exit_side = match get_exit_side(exit, old_width, old_height) {
        Some(side) => side,
        None => return false,
    };

    // Same side or opposite side: always valid
    if exit_side == drag_side || is_opposite_side(exit_side, drag_side) {
        return true;
    }

    // Adjacent side: check if inner coordinate stays in bounds
    let x = exit.position.x as i32;
    let y = exit.position.y as i32;

    match exit_side {
        HandleSide::Top | HandleSide::Bottom => {
            // Exit on horizontal edge, check x coordinate
            let new_x = if drag_side == HandleSide::Left && delta < 0 {
                x - delta.abs()
            } else {
                x
            };
            new_x >= 0 && new_x < new_width as i32
        }
        HandleSide::Left | HandleSide::Right => {
            // Exit on vertical edge, check y coordinate
            let new_y = if drag_side == HandleSide::Top && delta < 0 {
                y - delta.abs()
            } else {
                y
            };
            new_y >= 0 && new_y < new_height as i32
        }
    }
}
