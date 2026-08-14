use bishop::prelude::*;
use engine_core::ecs::*;
use engine_core::game::{GameCtxMut, StartupMode};
use engine_core::storage::{get_startup_mode, set_startup_mode};
use engine_core::theme::with_theme;
use engine_core::ui::measure_text;
use engine_core::worlds::{RoomId, RoomLayer};
use widgets::constants::layout;
use widgets::*;

use crate::app::EditorMode;
use crate::gui::gui_constants::*;
use crate::gui::menu_bar::*;
use crate::gui::mode_selector::*;
use crate::gui::panel_text_color;
use crate::room::room_editor::{RoomEditor, RoomEditorMode, RoomSceneSubMode, ROOM_SCENE_SUB_MODES};
use crate::shared::scene_ui::inspector::{InspectorContext, InspectorHostAction};
use crate::tilemap::tilemap_editor::TILEMAP_SUB_MODES;
use crate::world::coord;

const MODE_SELECTOR_PADDING: f32 = 8.0;

#[derive(Clone, Copy)]
struct MergedPlayButtonLayout {
    play_x: f32,
    play_y: f32,
    mode_x: f32,
    mode_y: f32,
    divider_x: f32,
    divider_y: f32,
    divider_h: f32,
    width: f32,
}

fn merged_play_button_layout(
    rect: Rect,
    play_dims: TextDimensions,
    mode_dims: TextDimensions,
) -> MergedPlayButtonLayout {
    let play_x = rect.x + layout::WIDGET_PADDING;
    let (_, play_y) = menu_button_text_position(rect, play_dims);
    let divider_x = play_x + play_dims.width + layout::WIDGET_PADDING;
    let mode_x = divider_x + layout::WIDGET_PADDING;
    let mode_y = rect.y + (rect.h - mode_dims.height) / 2.0 + mode_dims.offset_y;

    MergedPlayButtonLayout {
        play_x,
        play_y,
        mode_x,
        mode_y,
        divider_x,
        divider_y: rect.y + 6.0,
        divider_h: rect.h - 12.0,
        width: play_dims.width + mode_dims.width + layout::WIDGET_PADDING * 4.0,
    }
}

fn parent_mode_icon_x(
    ctx: &WgpuContext,
    selector: &ModeSelector<RoomEditorMode>,
    mode: RoomEditorMode,
) -> f32 {
    let icon_size = MENU_PANEL_HEIGHT - 2.0 * MODE_SELECTOR_PADDING;
    let total_width =
        selector.options.len() as f32 * (icon_size + MODE_SELECTOR_PADDING) - MODE_SELECTOR_PADDING;
    let start_x = (ctx.screen_width() - total_width) / 2.0;
    let mode_index = selector
        .options
        .iter()
        .position(|candidate| *candidate == mode)
        .unwrap_or(0);

    start_x + mode_index as f32 * (icon_size + MODE_SELECTOR_PADDING)
}

impl RoomEditor {
    pub(crate) fn draw_room_ui(
        &mut self,
        ctx: &mut WgpuContext,
        game_ctx: &mut GameCtxMut,
        camera: &Camera2D,
    ) {
        ctx.set_default_camera();

        let Some((grid_size, current_room_id, room_has_back_layer)) = game_ctx
            .world
            .as_deref()
            .and_then(|world| {
                world.current_room().map(|room| {
                    (
                        world.grid_size,
                        room.id,
                        room.current_variant().layers.back.is_some(),
                    )
                })
            })
        else {
            return;
        };

        self.draw_coordinates(ctx, camera, grid_size);
        self.sub_mode_rect = None;

        match self.mode {
            RoomEditorMode::Tilemap => {
                self.draw_tilemap_ui(ctx, game_ctx, current_room_id, room_has_back_layer)
            }
            RoomEditorMode::Scene => {
                self.draw_scene_ui(ctx, game_ctx, current_room_id, room_has_back_layer)
            }
        }
    }

    pub(crate) fn draw_coordinates(&self, ctx: &mut WgpuContext, camera: &Camera2D, grid_size: f32) {
        let world_grid = coord::mouse_world_grid(ctx, camera, grid_size);
        let txt = format!("({:.0}, {:.0})", world_grid.x, world_grid.y);
        let txt_metrics = measure_text(ctx, &txt, layout::DEFAULT_FONT_SIZE_16);
        let margin = 10.0;
        let x = (ctx.screen_width() - txt_metrics.width) / 2.0;
        let y = ctx.screen_height() - margin;

        ctx.draw_text(&txt, x, y, layout::DEFAULT_FONT_SIZE_16, Color::BLUE);
    }

    fn draw_tilemap_ui(
        &mut self,
        ctx: &mut WgpuContext,
        game_ctx: &mut GameCtxMut,
        current_room_id: RoomId,
        room_has_back_layer: bool,
    ) {
        self.register_rect(draw_top_panel_full(ctx));

        let inspector_ctx = InspectorContext {
            command_mode: EditorMode::Room(current_room_id),
            show_linked_prefab_metadata: false,
            hide_room_only_components: false,
            selected_create_parent: None,
            game_name: None,
            event_tags: self.event_tags.clone(),
            room_zone_tool_active: false,
            room_zones_visible: self.show_interior_zones,
        };
        let _ = self.inspector.draw_active_pane(ctx, game_ctx, &inspector_ctx);

        let tilemap_icon_x = parent_mode_icon_x(ctx, &self.mode_selector, RoomEditorMode::Tilemap);
        let icon_size = MENU_PANEL_HEIGHT - 2.0 * MODE_SELECTOR_PADDING;
        let sub_strip_y = MODE_SELECTOR_PADDING + icon_size + 4.0;

        let bg_rect = draw_sub_mode_strip_background(
            ctx,
            tilemap_icon_x,
            sub_strip_y,
            TILEMAP_SUB_MODES.len(),
        );
        self.sub_mode_rect = Some(bg_rect);

        let (mode_rect, changed) = self.mode_selector.draw(ctx);
        if changed {
            self.set_mode(self.mode_selector.current);
        }
        self.draw_layer_toggle_button(
            ctx,
            &*game_ctx.ecs,
            current_room_id,
            room_has_back_layer,
            mode_rect,
        );

        let (sub_rect, sub_changed) = draw_sub_mode_strip(
            ctx,
            tilemap_icon_x,
            sub_strip_y,
            TILEMAP_SUB_MODES,
            &mut self.tilemap_sub_mode,
        );
        self.sub_mode_rect = Some(sub_rect);

        self.mode_selector.draw_tooltips(ctx);

        if sub_changed {
            self.set_tilemap_sub_mode(self.tilemap_sub_mode);
        }

        for sub_mode in TILEMAP_SUB_MODES.iter() {
            if let Some(shortcut_fn) = sub_mode.shortcut() {
                if shortcut_fn(ctx) && *sub_mode != self.tilemap_sub_mode {
                    self.set_tilemap_sub_mode(*sub_mode);
                }
            }
        }
    }

    fn draw_scene_ui(
        &mut self,
        ctx: &mut WgpuContext,
        game_ctx: &mut GameCtxMut,
        current_room_id: RoomId,
        room_has_back_layer: bool,
    ) {
        self.register_rect(draw_top_panel_full(ctx));

        let inspector_ctx = InspectorContext {
            command_mode: EditorMode::Room(current_room_id),
            show_linked_prefab_metadata: true,
            hide_room_only_components: false,
            selected_create_parent: None,
            game_name: None,
            event_tags: self.event_tags.clone(),
            room_zone_tool_active: self.scene_sub_mode == RoomSceneSubMode::Zones,
            room_zones_visible: self.show_interior_zones,
        };
        let inspector_output = self.inspector.draw_active_pane(ctx, game_ctx, &inspector_ctx);
        self.create_request = inspector_output.create_request;
        self.prefab_action_request = inspector_output.prefab_action;
        self.create_camera_request = inspector_output.create_camera_request;
        self.request_event_tags_refresh = inspector_output.refresh_event_tags;
        if let Some(host_action) = inspector_output.host_action {
            match host_action {
                InspectorHostAction::ToggleRoomZoneTool => self.toggle_zone_sub_mode(),
                InspectorHostAction::ToggleRoomZoneVisibility => {
                    self.toggle_interior_zone_visibility();
                }
                _ => {}
            }
        }

        let (mode_rect, changed) = self.mode_selector.draw(ctx);
        if changed {
            self.set_mode(self.mode_selector.current);
        }
        self.draw_layer_toggle_button(
            ctx,
            &*game_ctx.ecs,
            current_room_id,
            room_has_back_layer,
            mode_rect,
        );

        let scene_icon_x = parent_mode_icon_x(ctx, &self.mode_selector, RoomEditorMode::Scene);
        let icon_size = MENU_PANEL_HEIGHT - 2.0 * MODE_SELECTOR_PADDING;
        let sub_strip_y = MODE_SELECTOR_PADDING + icon_size + 4.0;
        let scene_sub_modes = if room_has_back_layer {
            ROOM_SCENE_SUB_MODES
        } else {
            &ROOM_SCENE_SUB_MODES[..2]
        };
        let bg_rect = draw_sub_mode_strip_background(
            ctx,
            scene_icon_x,
            sub_strip_y,
            scene_sub_modes.len(),
        );
        self.sub_mode_rect = Some(bg_rect);

        let (sub_rect, sub_changed) = draw_sub_mode_strip(
            ctx,
            scene_icon_x,
            sub_strip_y,
            scene_sub_modes,
            &mut self.scene_sub_mode,
        );
        self.sub_mode_rect = Some(sub_rect);

        let play_label = "Play";
        let startup_mode = get_startup_mode();
        let play_dims = measure_text(ctx, play_label, layout::HEADER_FONT_SIZE_20);
        let mode_dims = measure_text(ctx, &startup_mode.to_string(), layout::DEFAULT_FONT_SIZE_16);
        let play_width = merged_play_button_layout(
            Rect::new(0.0, 0.0, 0.0, BTN_HEIGHT),
            play_dims,
            mode_dims,
        )
        .width;
        let play_x = mode_rect.x + mode_rect.w + layout::WIDGET_SPACING;
        let play_rect = Rect::new(play_x, INSET, play_width, BTN_HEIGHT);

        let clicks = Button::new(play_rect, "")
            .plain()
            .allow_secondary_click()
            .show_clicks(ctx, with_theme(Button::map_theme));

        if clicks.primary {
            self.request_play = true;
        }

        if clicks.secondary {
            set_startup_mode(startup_mode.toggled());
        }

        draw_merged_play_button_label(ctx, play_rect, play_dims, mode_dims, startup_mode);
        self.register_rect(play_rect);

        self.mode_selector.draw_tooltips(ctx);

        if sub_changed {
            self.set_scene_sub_mode(self.scene_sub_mode);
        }
    }

    fn draw_layer_toggle_button(
        &mut self,
        ctx: &mut WgpuContext,
        ecs: &Ecs,
        room_id: RoomId,
        has_back_layer: bool,
        mode_rect: Rect,
    ) {
        if !has_back_layer {
            return;
        }

        let front_dims = measure_text(ctx, "Front", layout::HEADER_FONT_SIZE_20);
        let back_dims = measure_text(ctx, "Back", layout::HEADER_FONT_SIZE_20);
        let width = front_dims.width.max(back_dims.width) + layout::WIDGET_PADDING * 2.0;
        let rect = Rect::new(
            mode_rect.x - layout::WIDGET_SPACING - width,
            INSET,
            width,
            BTN_HEIGHT,
        );
        let label = match self.active_layer_state.active_layer {
            RoomLayer::Front => "Front",
            RoomLayer::Back => "Back",
        };

        if menu_button(ctx, rect, label, false, false) {
            self.toggle_active_layer(ecs, room_id, has_back_layer);
        }
        self.register_rect(rect);
    }
}

fn draw_merged_play_button_label(
    ctx: &mut WgpuContext,
    rect: Rect,
    play_dims: TextDimensions,
    mode_dims: TextDimensions,
    startup_mode: StartupMode,
) {
    let layout = merged_play_button_layout(rect, play_dims, mode_dims);

    let text_color = panel_text_color();
    ctx.draw_text(
        "Play",
        layout.play_x,
        layout.play_y,
        layout::HEADER_FONT_SIZE_20,
        text_color,
    );
    ctx.draw_line(
        layout.divider_x,
        layout.divider_y,
        layout.divider_x,
        layout.divider_y + layout.divider_h,
        1.0,
        text_color,
    );
    ctx.draw_text(
        &startup_mode.to_string(),
        layout.mode_x,
        layout.mode_y,
        layout::DEFAULT_FONT_SIZE_16,
        text_color,
    );
}
