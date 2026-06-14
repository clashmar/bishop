use crate::app::EditorMode;
use crate::commands::game::EditWorldCmd;
use crate::editor_assets::assets::{world_entry_icon, world_exit_icon};
use crate::editor_global::{push_command, push_toast};
use crate::gui::gui_constants;
use crate::gui::menu_bar::draw_top_panel_full;
use crate::shared::scene_ui::inspector::{InspectorContext, InspectorHostAction};
use crate::world::coord::{mouse_world_grid, mouse_world_pos, rect_from_points, snap_to_grid};
use crate::world::world_editor::{HOVER_LINE_THICKNESS, LINE_THICKNESS_MULTIPLIER, WorldEditor, WorldEditorMode};
use bishop::prelude::*;
use engine_core::ecs::Transform;
use engine_core::game::Game;
use engine_core::theme::with_theme;
use engine_core::ui::measure_text;
use engine_core::worlds::*;
use ::widgets::constants::layout;

const ROOM_LINE_INSET: f32 = 1.0;

impl WorldEditor {
    pub fn draw_rooms(
        &self,
        ctx: &mut WgpuContext,
        camera: &Camera2D,
        rooms: &[Room],
        grid_size: f32,
    ) {
        for room in rooms {
            let rect = scaled_room_rect(room, grid_size);
            let inset = ROOM_LINE_INSET * grid_size;
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

    pub(super) fn draw_exits(&self, ctx: &mut WgpuContext, rooms: &[Room], grid_size: f32) {
        for room in rooms {
            for exit in &room.exits {
                let exit_world_coord = (room.position / grid_size) + exit.position;
                let color = if exit.target_room_id.is_some() {
                    Color::GREEN
                } else {
                    Color::RED
                };
                self.draw_exit_marker(ctx, exit_world_coord, exit.direction, color, grid_size);
            }
        }
    }

    pub(super) fn draw_hovered_room(
        &self,
        ctx: &mut WgpuContext,
        camera: &Camera2D,
        rooms: &[Room],
        grid_size: f32,
    ) {
        let world_mouse = mouse_world_pos(ctx, camera);
        for room in rooms {
            let rect = scaled_room_rect(room, grid_size);
            if rect.contains(world_mouse) {
                let inset = ROOM_LINE_INSET * grid_size;
                let color = match self.mode {
                    WorldEditorMode::Delete => with_theme(|t| t.danger.with_alpha(0.5)),
                    _ => with_theme(|t| t.primary.with_alpha(0.5)),
                };
                ctx.draw_rectangle(
                    rect.x + inset / 2.0,
                    rect.y + inset / 2.0,
                    rect.w - inset,
                    rect.h - inset,
                    color,
                );
                break;
            }
        }
    }

    pub(super) fn draw_room_names(
        &self,
        ctx: &mut WgpuContext,
        camera: &Camera2D,
        rooms: &[Room],
        grid_size: f32,
    ) {
        ctx.set_default_camera();

        for room in rooms {
            let rect = scaled_room_rect(room, grid_size);
            let screen_pos = camera.world_to_screen(
                rect.top_left() + rect.size() / 2.0,
                ctx.screen_width(),
                ctx.screen_height(),
            );
            let base_font_size: f32 = 40.0;
            let room_scale = (rect.w + rect.h) / 2.0 / 60.0;
            let zoom_factor = camera.zoom.x * 100.0;
            let font_size = (base_font_size * room_scale * zoom_factor).clamp(10.0, 200.0);
            let rotation = if rect.h > rect.w {
                std::f32::consts::FRAC_PI_2
            } else {
                0.0
            };
            let dims = ctx.measure_text(&room.name, font_size);
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

        ctx.set_camera(camera);
    }

    pub(super) fn draw_placing_preview(
        &self,
        ctx: &mut WgpuContext,
        camera: &Camera2D,
        rooms: &[Room],
        grid_size: f32,
    ) {
        if let (Some(start), Some(end)) = (self.placing_start, self.placing_end) {
            let (top_left, size) = rect_from_points(start, end);
            let color = if self.intersects_existing_room(rooms, top_left, size, grid_size) {
                with_theme(|t| t.danger.with_alpha(0.5))
            } else {
                with_theme(|t| t.accent.with_alpha(0.5))
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
                    with_theme(|t| t.danger.with_alpha(0.5))
                } else {
                    with_theme(|t| t.accent.with_alpha(0.5))
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

    /// Draws the grid coordinate of the mouse position at the bottom of the screen.
    pub fn draw_coordinates(&self, ctx: &mut WgpuContext, camera: &Camera2D, grid_size: f32) {
        let world_grid = mouse_world_grid(ctx, camera, grid_size);
        let txt = format!("({:.0}, {:.0})", world_grid.x, world_grid.y);
        let txt_metrics = measure_text(ctx, &txt, layout::DEFAULT_FONT_SIZE_16);
        let margin = 10.0;
        let x = (ctx.screen_width() - txt_metrics.width) / 2.0;
        let y = ctx.screen_height() - margin;
        ctx.draw_text(&txt, x, y, layout::DEFAULT_FONT_SIZE_16, Color::BLACK);
    }

    pub(super) fn draw_ui(
        &mut self, 
        ctx: &mut WgpuContext, 
        camera: &Camera2D, 
        game: &mut Game, 
        world_id: WorldId
    ) {
        self.active_rects.clear();

        ctx.set_default_camera();

        self.register_rect(draw_top_panel_full(ctx));

        if self.mode_selector.draw(ctx).1 {
            self.mode = self.mode_selector.current;
        }
        self.mode_selector.draw_tooltips(ctx);

        let inspector_rect = Rect::new(
            ctx.screen_width() - gui_constants::inspector::WIDTH,
            0.0,
            gui_constants::inspector::WIDTH,
            ctx.screen_height(),
        );
        self.inspector.set_rect(inspector_rect);

        let inspector_output = {
            let mut game_ctx = game.ctx_mut();
            self.inspector.draw_world_pane(
                ctx,
                &mut game_ctx,
                &InspectorContext {
                    command_mode: EditorMode::World(world_id),
                    show_linked_prefab_metadata: false,
                    hide_room_only_components: true,
                    selected_create_parent: None,
                    game_name: None,
                    event_tags: Vec::new(),
                },
            )
        };

        if let Some(host_action) = inspector_output.host_action {
            match host_action {
                InspectorHostAction::RenameWorld(name) => {
                    let unique = game.unique_world_name(&name, Some(world_id));
                    if unique != name {
                        push_toast(format!("'{}' is already taken, renamed to '{}'", name, unique), 3.0);
                    }
                    push_command(Box::new(EditWorldCmd::new(world_id, Some(unique), None)));
                }
                InspectorHostAction::FocusWorldEditor(entity) => {
                    self.inspector.select_entity(entity);
                    let grid_size = game.get_world(world_id).map(|w| w.grid_size).unwrap_or(16.0);
                    if let Some(t) = game.ecs.get::<Transform>(entity).map(|t| t.position) {
                        self.pending_camera_focus = Some(snap_to_tile(t, grid_size));
                    }
                }
                _ => {}
            }
        }

        ctx.set_camera(camera);
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
}

/// Returns the screen rect for a room scaled by grid_size.
pub(super) fn scaled_room_rect(room: &Room, grid_size: f32) -> Rect {
    let size = room.size;
    Rect::new(
        room.position.x,
        room.position.y,
        size.x * grid_size,
        size.y * grid_size,
    )
}

/// Draws entry and exit icons at their world-canvas positions, sized one grid square.
pub(super) fn draw_navigation_icons(
    ctx: &mut WgpuContext,
    entry_positions: &[Vec2],
    exit_positions: &[Vec2],
    grid_size: f32,
) {
    for &pos in entry_positions {
        ctx.draw_texture_ex(
            world_entry_icon(),
            pos.x,
            pos.y,
            Color::WHITE,
            DrawTextureParams {
                dest_size: Some(vec2(grid_size, grid_size)),
                ..Default::default()
            },
        );
    }
    for &pos in exit_positions {
        ctx.draw_texture_ex(
            world_exit_icon(),
            pos.x,
            pos.y,
            Color::WHITE,
            DrawTextureParams {
                dest_size: Some(vec2(grid_size, grid_size)),
                ..Default::default()
            },
        );
    }
}

/// Snaps a world-space pixel position to the top-left of the tile it occupies.
pub(super) fn snap_to_tile(pos: Vec2, grid_size: f32) -> Vec2 {
    Vec2::new(
        (pos.x / grid_size).floor() * grid_size,
        ((pos.y - 1.0) / grid_size).floor() * grid_size,
    )
}
