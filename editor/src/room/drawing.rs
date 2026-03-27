// editor/src/room/drawing.rs
use crate::app::camera_controller::*;
use crate::gui::gui_constants::*;
use crate::gui::menu_bar::*;
use crate::gui::mode_selector::*;
use crate::room::room_editor::*;
use crate::tilemap::tilemap_editor::TILEMAP_SUB_MODES;
use crate::world::coord;
use bishop::prelude::*;
use engine_core::prelude::*;

const PLACEHOLDER_OPACITY: f32 = 0.5;
fn thickness(grid_size: f32) -> f32 {
    (grid_size * 0.1).max(1.0)
}

impl RoomEditor {
    /// Draw static UI for the scene editor
    pub fn draw_ui(&mut self, ctx: &mut WgpuContext, game_ctx: &mut GameCtxMut, camera: &Camera2D) {
        // Reset to static camera
        ctx.set_default_camera();

        self.draw_coordinates(ctx, camera, game_ctx.cur_world.grid_size);

        // Clear sub-mode rect at start of frame
        self.sub_mode_rect = None;

        match self.mode {
            RoomEditorMode::Tilemap => {
                // Calculate sub-mode strip position
                let tilemap_icon_index = self
                    .mode_selector
                    .options
                    .iter()
                    .position(|m| *m == RoomEditorMode::Tilemap)
                    .unwrap_or(0);

                const PADDING: f32 = 8.0;
                let icon_size = MENU_PANEL_HEIGHT - 2.0 * PADDING;
                let total_width =
                    self.mode_selector.options.len() as f32 * (icon_size + PADDING) - PADDING;
                let start_x = (ctx.screen_width() - total_width) / 2.0;
                let tilemap_icon_x = start_x + tilemap_icon_index as f32 * (icon_size + PADDING);
                let sub_strip_y = PADDING + icon_size + 4.0;

                // Draw sub-mode strip background first so tooltips appear on top
                let bg_rect = draw_sub_mode_strip_background(
                    ctx,
                    tilemap_icon_x,
                    sub_strip_y,
                    TILEMAP_SUB_MODES.len(),
                );
                self.sub_mode_rect = Some(bg_rect);

                // Mode selector
                let (_mode_rect, changed) = self.mode_selector.draw(ctx);
                if changed {
                    self.mode = self.mode_selector.current;
                }

                // Draw sub-mode strip icons
                let (sub_rect, sub_changed) = draw_sub_mode_strip(
                    ctx,
                    tilemap_icon_x,
                    sub_strip_y,
                    TILEMAP_SUB_MODES,
                    &mut self.tilemap_sub_mode,
                );

                self.sub_mode_rect = Some(sub_rect);

                // Draw tooltips last so they appear on top of everything
                self.mode_selector.draw_tooltips(ctx);

                if sub_changed {
                    self.tilemap_editor.mode = self.tilemap_sub_mode;
                }

                // Handle sub-mode keyboard shortcuts
                for sub_mode in TILEMAP_SUB_MODES.iter() {
                    if let Some(shortcut_fn) = sub_mode.shortcut() {
                        if shortcut_fn(ctx) && *sub_mode != self.tilemap_sub_mode {
                            self.tilemap_sub_mode = *sub_mode;
                            self.tilemap_editor.mode = self.tilemap_sub_mode;
                        }
                    }
                }
            }
            RoomEditorMode::Scene => {
                // Top menu background
                self.register_rect(draw_top_panel_full(ctx));

                // Draw inspector
                self.create_entity_requested = self.inspector.draw(ctx, game_ctx);

                // Mode selector (menu bar)
                let (mode_rect, changed) = self.mode_selector.draw(ctx);
                if changed {
                    self.mode = self.mode_selector.current;
                }

                // Play‑test button (menu bar)
                let play_label = "Play";
                let play_width =
                    measure_text(ctx, play_label, HEADER_FONT_SIZE_20).width + WIDGET_PADDING * 2.0;
                let play_x = mode_rect.x + mode_rect.w + WIDGET_SPACING;
                let play_rect = Rect::new(play_x, INSET, play_width, BTN_HEIGHT);

                if menu_button(ctx, play_rect, play_label, false) {
                    self.request_play = true;
                }
            }
        }
    }

    /// Draw the cursor coordinates in world space.
    pub fn draw_coordinates(&self, ctx: &mut WgpuContext, camera: &Camera2D, grid_size: f32) {
        let world_grid = coord::mouse_world_grid(ctx, camera, grid_size);

        let txt = format!("({:.0}, {:.0})", world_grid.x, world_grid.y,);

        let txt_metrics = measure_text(ctx, &txt, DEFAULT_FONT_SIZE_16);
        let margin = 10.0;

        let x = (ctx.screen_width() - txt_metrics.width) / 2.0;
        let y = ctx.screen_height() - margin;

        ctx.draw_text(&txt, x, y, DEFAULT_FONT_SIZE_16, Color::BLUE);
    }

    /// Draw viewport rectangles for all cameras in the room when a camera is selected.
    /// The selected camera is drawn in yellow, others in pink.
    pub fn draw_camera_viewport(
        &self,
        ctx: &mut WgpuContext,
        editor_cam: &Camera2D,
        ecs: &Ecs,
        selected: Entity,
        room_id: RoomId,
    ) {
        // Only draw viewports if the selected entity is a camera
        if !ecs.has::<RoomCamera>(selected) {
            return;
        }

        let cam_store = ecs.get_store::<RoomCamera>();
        let pos_store = ecs.get_store::<Transform>();
        let room_store = ecs.get_store::<CurrentRoom>();

        let editor_scalar = EditorCameraController::scalar_zoom(ctx, editor_cam);
        const BASE_THICKNESS: f32 = 0.25;
        const THICKNESS_SCALE: f32 = 0.01;
        let thickness = BASE_THICKNESS * (THICKNESS_SCALE / editor_scalar).max(1.0);

        let screen_w = ctx.screen_width();
        let screen_h = ctx.screen_height();
        let bl = editor_cam.screen_to_world(vec2(0.0, 0.0), screen_w, screen_h);
        let tr = editor_cam.screen_to_world(vec2(screen_w, screen_h), screen_w, screen_h);
        let editor_w = (tr.x - bl.x).abs();
        let editor_h = (tr.y - bl.y).abs();

        // Collect all cameras in this room
        for (entity, room_cam) in cam_store.data.iter() {
            // Only draw cameras in this room
            if let Some(CurrentRoom(id)) = room_store.get(*entity) {
                if *id != room_id {
                    continue;
                }
            } else {
                continue;
            }

            let pos = match pos_store.get(*entity) {
                Some(p) => p.position,
                None => continue,
            };

            let factor_x = editor_cam.zoom.x / room_cam.zoom.x;
            let factor_y = editor_cam.zoom.y / room_cam.zoom.y;

            let viewport_w = editor_w * factor_x;
            let viewport_h = editor_h * factor_y;

            let half = vec2(viewport_w, viewport_h) * 0.5;
            let top_left = pos - half;

            // Selected camera is yellow, others are dimmer cyan
            let color = if *entity == selected {
                Color::YELLOW
            } else {
                Color::PINK
            };

            ctx.draw_rectangle_lines(
                top_left.x, top_left.y, viewport_w, viewport_h, thickness, color,
            );
        }
    }
}

/// Highlight a selected entity with a colored outline.
pub fn highlight_selected_entity<C: BishopContext>(
    ctx: &mut C,
    ecs: &Ecs,
    entity: Entity,
    asset_manager: &mut AssetManager,
    color: Color,
    grid_size: f32,
) {
    let transform = match ecs.get_store::<Transform>().get(entity) {
        Some(t) => t,
        None => return,
    };

    // If this is a proxy, use Player's visual components for dimensions
    let visual_entity = if ecs.has::<PlayerProxy>(entity) {
        ecs.get_player_entity().unwrap_or(entity)
    } else {
        entity
    };

    let size = entity_dimensions(ecs, asset_manager, visual_entity, grid_size);
    let draw_pos = pivot_adjusted_position(transform.position, size, transform.pivot);

    ctx.draw_rectangle_lines(
        draw_pos.x,
        draw_pos.y,
        size.x,
        size.y,
        thickness(grid_size) * 0.25,
        color,
    );
}

/// Draw the outline of the collider for an entity if it has one.
pub fn draw_collider(ctx: &mut WgpuContext, ecs: &Ecs, entity: Entity) {
    if let Some((width, height)) = ecs
        .get_store::<Collider>()
        .get(entity)
        .filter(|c| c.width > 0.0 && c.height > 0.0)
        .map(|c| (c.width, c.height))
    {
        let transform = match ecs.get_store::<Transform>().get(entity) {
            Some(t) => t,
            None => return,
        };

        // Apply pivot offset to collider position
        let draw_pos =
            pivot_adjusted_position(transform.position, vec2(width, height), transform.pivot);
        ctx.draw_rectangle_lines(draw_pos.x, draw_pos.y, width, height, 1.0, Color::PINK);
    }
}

/// Draw an icon for a `RoomCamera`.
pub fn draw_camera_placeholders(ctx: &mut WgpuContext, ecs: &Ecs, room_id: RoomId, grid_size: f32) {
    let cam_store = ecs.get_store::<RoomCamera>();
    let pos_store = ecs.get_store::<Transform>();
    let room_store = ecs.get_store::<CurrentRoom>();

    let positions: Vec<Vec2> = cam_store
        .data
        .iter()
        .filter_map(|(entity, _room_cam)| {
            let cur_room = room_store.get(*entity)?;
            if cur_room.0 != room_id {
                return None;
            }
            let pos = pos_store.get(*entity)?;
            Some(pos.position)
        })
        .collect();

    for pos in positions {
        // Offset the camera placeholder
        let half_tile = grid_size * 0.5;
        let body = Rect::new(pos.x - half_tile, pos.y - half_tile, grid_size, grid_size);

        let green = Color::new(0.0, 0.89, 0.19, PLACEHOLDER_OPACITY);
        let blue = Color::new(0.0, 0.47, 0.95, PLACEHOLDER_OPACITY);
        let red = Color::new(0.9, 0.16, 0.22, PLACEHOLDER_OPACITY);

        ctx.draw_rectangle_lines(body.x, body.y, body.w, body.h, thickness(grid_size), green);

        let finder_w = grid_size * 0.3;
        let finder_h = grid_size * 0.6;
        let finder = Rect::new(
            body.x + thickness(grid_size),
            body.y + (body.h - finder_h) / 2.0,
            finder_w,
            finder_h,
        );
        ctx.draw_rectangle_lines(
            finder.x,
            finder.y,
            finder.w,
            finder.h,
            thickness(grid_size) * 0.75,
            blue,
        );

        let lens_radius = grid_size * 0.1;
        let lens_center = vec2(
            body.x + body.w - lens_radius * 2.0 - thickness(grid_size),
            body.y + body.h / 2.0,
        );
        ctx.draw_circle_lines(
            lens_center.x,
            lens_center.y,
            lens_radius,
            thickness(grid_size) * 0.75,
            red,
        );
    }
}

/// Draw an icon for a `Light` that has no other visual component.
pub fn draw_light_placeholders(ctx: &mut WgpuContext, ecs: &Ecs, room_id: RoomId, grid_size: f32) {
    let room_store = ecs.get_store::<CurrentRoom>();
    for (entity, _light) in ecs.get_store::<Light>().data.iter() {
        // Only draw placeholders in this room
        if let Some(CurrentRoom(id)) = room_store.get(*entity) {
            if *id != room_id {
                continue;
            }
        }

        // Don't draw if there is a Sprite or Animation component
        if ecs.has_any::<(Sprite, Animation)>(*entity) {
            continue;
        }

        if let Some(position) = ecs.get_store::<Transform>().get(*entity) {
            let pos = position.position;

            let half_tile = grid_size * 0.5;
            let body = Rect::new(pos.x - half_tile, pos.y - half_tile, grid_size, grid_size);

            let cyan = Color::new(0.0, 0.78, 0.78, PLACEHOLDER_OPACITY);
            let yellow = Color::new(0.94, 0.86, 0.0, PLACEHOLDER_OPACITY);

            // Outer square
            ctx.draw_rectangle_lines(body.x, body.y, body.w, body.h, thickness(grid_size), cyan);

            // Lens
            let lens_radius = grid_size * 0.2;
            let lens_center = vec2(body.x + body.w / 2., body.y + body.h / 2.);

            ctx.draw_circle_lines(
                lens_center.x,
                lens_center.y,
                lens_radius,
                thickness(grid_size) * 0.75,
                yellow,
            );
        }
    }
}

/// Draw a placeholder for a `Glow` that has no other visual component.
pub fn draw_glow_placeholders(
    ctx: &mut WgpuContext,
    ecs: &Ecs,
    asset_manager: &mut AssetManager,
    room_id: RoomId,
    grid_size: f32,
) {
    let room_store = ecs.get_store::<CurrentRoom>();
    for (entity, glow) in ecs.get_store::<Glow>().data.iter() {
        // Only draw placeholders in this room
        if let Some(CurrentRoom(id)) = room_store.get(*entity) {
            if *id != room_id {
                continue;
            }
        }

        // Don't draw if there is a Sprite or Animation component
        if ecs.has_any::<(Sprite, Animation)>(*entity) {
            continue;
        }

        if let Some(position) = ecs.get_store::<Transform>().get(*entity) {
            let mut pos = position.position;

            if let Some((w, h)) = asset_manager.texture_size(glow.sprite_id) {
                pos += vec2((w / 2.) - grid_size / 2., (h / 2.) - grid_size / 2.);
            }

            let body = Rect::new(pos.x, pos.y, grid_size, grid_size);

            let cyan = Color::new(0.0, 0.78, 0.78, PLACEHOLDER_OPACITY);
            let yellow = Color::new(0.94, 0.86, 0.0, PLACEHOLDER_OPACITY);

            // Outer square
            ctx.draw_rectangle_lines(body.x, body.y, body.w, body.h, thickness(grid_size), cyan);

            // Lens
            let lens_radius = grid_size * 0.2;
            let lens_center = vec2(body.x + body.w / 2., body.y + body.h / 2.);

            ctx.draw_circle_lines(
                lens_center.x,
                lens_center.y,
                lens_radius,
                thickness(grid_size) * 0.75,
                yellow,
            );
        }
    }
}

/// Draws a small white dot at the pivot point of the selected entity.
pub fn draw_pivot_marker(ctx: &mut WgpuContext, ecs: &Ecs, entity: Entity) {
    let transform = match ecs.get_store::<Transform>().get(entity) {
        Some(t) => t,
        None => return,
    };

    const PIVOT_RADIUS: f32 = 1.0;
    ctx.draw_circle(
        transform.position.x,
        transform.position.y,
        PIVOT_RADIUS,
        Color::WHITE,
    );
}

/// Returns true if the entity is a pure placeholder (Camera or Light without visible sprites).
pub fn is_pure_placeholder(ecs: &Ecs, entity: Entity) -> bool {
    ecs.has::<RoomCamera>(entity)
        || (ecs.has::<Light>(entity) && !ecs.has_any::<(Sprite, Animation, CurrentFrame)>(entity))
}

/// Draw a thin circle showing the interaction range for each `Interactable` entity in the room.
pub fn draw_interactable_ranges(ctx: &mut WgpuContext, ecs: &Ecs, room_id: RoomId, grid_size: f32) {
    let room_store = ecs.get_store::<CurrentRoom>();
    let violet = Color::new(0.75, 0.25, 1.0, 0.55);

    for (entity, interactable) in ecs.get_store::<Interactable>().data.iter() {
        if let Some(CurrentRoom(id)) = room_store.get(*entity) {
            if *id != room_id {
                continue;
            }
        }

        if let Some(transform) = ecs.get_store::<Transform>().get(*entity) {
            let pos = transform.position;
            ctx.draw_circle_lines(
                pos.x,
                pos.y,
                interactable.range,
                thickness(grid_size) * 0.25,
                violet,
            );
        }
    }
}

/// Draw exit arrows for all exits in the room.
pub fn draw_exit_placeholders(
    ctx: &mut WgpuContext,
    exits: &[Exit],
    room_position: Vec2,
    grid_size: f32,
) {
    for exit in exits {
        let position = exit.position * grid_size + room_position;
        draw_exit_arrow(ctx, position, exit.direction, grid_size);
    }
}

/// Draw all camera viewports in a room.
pub fn draw_all_camera_viewports(
    ctx: &mut WgpuContext,
    editor_cam: &Camera2D,
    ecs: &Ecs,
    room_id: RoomId,
) {
    let cam_store = ecs.get_store::<RoomCamera>();
    let pos_store = ecs.get_store::<Transform>();
    let room_store = ecs.get_store::<CurrentRoom>();

    let editor_scalar = EditorCameraController::scalar_zoom(ctx, editor_cam);
    const BASE_THICKNESS: f32 = 0.5;
    const THICKNESS_SCALE: f32 = 0.01;
    let thickness = BASE_THICKNESS * (THICKNESS_SCALE / editor_scalar).max(1.0);

    let screen_w = ctx.screen_width();
    let screen_h = ctx.screen_height();
    let bl = editor_cam.screen_to_world(vec2(0.0, 0.0), screen_w, screen_h);
    let tr = editor_cam.screen_to_world(vec2(screen_w, screen_h), screen_w, screen_h);
    let editor_w = (tr.x - bl.x).abs();
    let editor_h = (tr.y - bl.y).abs();

    for (entity, room_cam) in cam_store.data.iter() {
        if let Some(CurrentRoom(id)) = room_store.get(*entity) {
            if *id != room_id {
                continue;
            }
        } else {
            continue;
        }

        let pos = match pos_store.get(*entity) {
            Some(p) => p.position,
            None => continue,
        };

        let factor_x = editor_cam.zoom.x / room_cam.zoom.x;
        let factor_y = editor_cam.zoom.y / room_cam.zoom.y;

        let viewport_w = editor_w * factor_x;
        let viewport_h = editor_h * factor_y;

        let half = vec2(viewport_w, viewport_h) * 0.5;
        let top_left = pos - half;

        ctx.draw_rectangle_lines(
            top_left.x,
            top_left.y,
            viewport_w,
            viewport_h,
            thickness,
            Color::PINK,
        );
    }
}

/// Draw a semi-transparent arrow at the given position indicating exit direction.
pub fn draw_exit_arrow(
    ctx: &mut WgpuContext,
    position: Vec2,
    direction: ExitDirection,
    grid_size: f32,
) {
    draw_exit_arrow_colored(ctx, position, direction, grid_size, HIGHLIGHT_GREEN);
}

/// Draw an arrow for an adjacent room's exit (pink color to distinguish from current room).
pub fn draw_adjacent_exit_arrow(
    ctx: &mut WgpuContext,
    position: Vec2,
    direction: ExitDirection,
    grid_size: f32,
) {
    draw_exit_arrow_colored(ctx, position, direction, grid_size, Color::YELLOW);
}

/// Draw an exit arrow with a specified color.
fn draw_exit_arrow_colored(
    ctx: &mut WgpuContext,
    position: Vec2,
    direction: ExitDirection,
    grid_size: f32,
    color: Color,
) {
    let x = position.x;
    let y = position.y;

    let arrow_center = vec2(x + grid_size / 2.0, y + grid_size / 2.0);

    let offsets = match direction {
        ExitDirection::Up => [vec2(0.0, -1.0), vec2(-1.0, 1.0), vec2(1.0, 1.0)],
        ExitDirection::Down => [vec2(0.0, 1.0), vec2(-1.0, -1.0), vec2(1.0, -1.0)],
        ExitDirection::Left => [vec2(-1.0, 0.0), vec2(1.0, -1.0), vec2(1.0, 1.0)],
        ExitDirection::Right => [vec2(1.0, 0.0), vec2(-1.0, -1.0), vec2(-1.0, 1.0)],
    };

    ctx.draw_triangle(
        arrow_center + offsets[0] * grid_size / 3.0,
        arrow_center + offsets[1] * grid_size / 3.0,
        arrow_center + offsets[2] * grid_size / 3.0,
        color,
    );
}
