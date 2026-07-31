use bishop::prelude::*;
use engine_core::assets::*;
use engine_core::constants::world as world_constants;
use engine_core::ecs::*;
use engine_core::game::{GameCtxMut, StartupMode};
use engine_core::rendering::{draw_collider, outline_thickness, pivot_adjusted_position, resolve_visual_entity, ENTITY_OUTLINE_SCALE};
use engine_core::storage::*;
use engine_core::theme::with_theme;
use engine_core::ui::measure_text;
use engine_core::worlds::*;
use widgets::constants::layout;
use widgets::*;

use crate::app::control::camera_controller::EditorCameraController;
use crate::app::EditorMode;
use crate::editor_assets::assets::{camera_icon, entity_icon, entry_icon, exit_icon, portal_icon};
use crate::gui::gui_constants::*;
use crate::gui::inspector::collider_module::edit::{compute_handles, is_collider_edit_active_for};
use crate::gui::menu_bar::*;
use crate::gui::mode_selector::*;
use crate::gui::panel_text_color;
use crate::room::prefab_preview::{build_prefab_preview, PrefabPreviewVisual};
use crate::room::room_editor::*;
use crate::room::selection::{entity_selection_rect, snap_room_drag_position};
use crate::shared::entity_icon::{
    draw_camera_icon, draw_glow_placeholder, draw_light_placeholder, resolve_entity_visual,
    EntityVisual, PLACEHOLDER_OPACITY,
};
use crate::shared::scene_ui::inspector::InspectorContext;
use crate::tilemap::tilemap_editor::TILEMAP_SUB_MODES;
use crate::world::coord;

const MODE_SELECTOR_PADDING: f32 = 8.0;
const PREFAB_GHOST_OPACITY: f32 = 0.55;

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
    /// Draw static UI for the active room editor.
    pub fn draw_ui(&mut self, ctx: &mut WgpuContext, game_ctx: &mut GameCtxMut, camera: &Camera2D) {
        // Reset to static camera
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

        // Clear sub-mode rect at start of frame
        self.sub_mode_rect = None;

        match self.mode {
            RoomEditorMode::Tilemap => {
                self.register_rect(draw_top_panel_full(ctx));

                let inspector_ctx = InspectorContext {
                    command_mode: EditorMode::Room(current_room_id),
                    show_linked_prefab_metadata: false,
                    hide_room_only_components: false,
                    selected_create_parent: None,
                    game_name: None,
                    event_tags: self.event_tags.clone(),
                    room_zone_tool_active: false,
                };
                let _ = self.inspector.draw_active_pane(ctx, game_ctx, &inspector_ctx);

                let tilemap_icon_x =
                    parent_mode_icon_x(ctx, &self.mode_selector, RoomEditorMode::Tilemap);
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
            RoomEditorMode::Scene => {
                // Top menu background
                self.register_rect(draw_top_panel_full(ctx));

                // Draw inspector
                let inspector_ctx = InspectorContext {
                    command_mode: EditorMode::Room(current_room_id),
                    show_linked_prefab_metadata: true,
                    hide_room_only_components: false,
                    selected_create_parent: None,
                    game_name: None,
                    event_tags: self.event_tags.clone(),
                    room_zone_tool_active: self.scene_sub_mode == RoomSceneSubMode::Zones,
                };
                let inspector_output = self.inspector.draw_active_pane(ctx, game_ctx, &inspector_ctx);
                self.create_request = inspector_output.create_request;
                self.prefab_action_request = inspector_output.prefab_action;
                self.create_camera_request = inspector_output.create_camera_request;
                self.request_event_tags_refresh = inspector_output.refresh_event_tags;
                if inspector_output.toggle_room_zone_tool {
                    self.toggle_zone_sub_mode();
                }

                // Mode selector (menu bar)
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

                let scene_icon_x =
                    parent_mode_icon_x(ctx, &self.mode_selector, RoomEditorMode::Scene);
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

                // Play‑test button (menu bar)
                let play_label = "Play";
                let startup_mode = get_startup_mode();
                let play_dims = measure_text(ctx, play_label, layout::HEADER_FONT_SIZE_20);
                let mode_dims =
                    measure_text(ctx, &startup_mode.to_string(), layout::DEFAULT_FONT_SIZE_16);
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
        }
    }

    /// Draw the cursor coordinates in world space.
    pub fn draw_coordinates(&self, ctx: &mut WgpuContext, camera: &Camera2D, grid_size: f32) {
        let world_grid = coord::mouse_world_grid(ctx, camera, grid_size);

        let txt = format!("({:.0}, {:.0})", world_grid.x, world_grid.y,);

        let txt_metrics = measure_text(ctx, &txt, layout::DEFAULT_FONT_SIZE_16);
        let margin = 10.0;

        let x = (ctx.screen_width() - txt_metrics.width) / 2.0;
        let y = ctx.screen_height() - margin;

        ctx.draw_text(&txt, x, y, layout::DEFAULT_FONT_SIZE_16, Color::BLUE);
    }

    /// Draw viewport rectangles for all cameras in the room when a camera is selected.
    pub fn draw_camera_viewports(
        &self,
        ctx: &mut WgpuContext,
        editor_cam: &Camera2D,
        ecs: &Ecs,
        selected: Entity,
        room_id: RoomId,
        layer: RoomLayer,
    ) {
        // Only draw viewports if the selected entity is a camera
        if !ecs.has::<RoomCamera>(selected) {
            return;
        }

        let cam_store = ecs.get_store::<RoomCamera>();
        let pos_store = ecs.get_store::<Transform>();

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

        for &entity in ecs.entities_in_room_layer(room_id, layer) {
            ecs.assert_room_membership(room_id, entity);

            let Some(room_cam) = cam_store.get(entity) else {
                continue;
            };
            let Some(pos) = pos_store.get(entity).map(|transform| transform.position) else {
                continue;
            };

            let factor_x = editor_cam.zoom.x / room_cam.zoom.x;
            let factor_y = editor_cam.zoom.y / room_cam.zoom.y;

            let viewport_w = editor_w * factor_x;
            let viewport_h = editor_h * factor_y;

            let half = vec2(viewport_w, viewport_h) * 0.5;
            let top_left = pos - half;

            let color = if entity == selected {
                with_theme(|t| t.highlight)
            } else {
                with_theme(|t| t.accent)
            };

            ctx.draw_rectangle_lines(
                top_left.x, top_left.y, viewport_w, viewport_h, thickness, color,
            );
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

/// Highlight a selected entity with a colored outline.
pub fn highlight_selected_entity<C: BishopContext>(
    ctx: &mut C,
    ecs: &Ecs,
    entity: Entity,
    sprite_manager: &mut SpriteManager,
    grid_size: f32,
) {
    let transform = match ecs.get_store::<Transform>().get(entity) {
        Some(t) => t,
        None => return,
    };

    let (draw_pos, size) =
        entity_selection_rect(entity, transform.position, ecs, sprite_manager, grid_size);

    let color = with_theme(|t| t.highlight);
    ctx.draw_rectangle_lines(
        draw_pos.x,
        draw_pos.y,
        size.x,
        size.y,
        outline_thickness(grid_size) * ENTITY_OUTLINE_SCALE,
        color,
    );
}

pub(crate) fn draw_prefab_stamp_ghost(
    ctx: &mut WgpuContext,
    camera: &Camera2D,
    asset_registry: &mut AssetRegistry,
    sprite_manager: &mut SpriteManager,
    prefab: &PrefabAsset,
    grid_size: f32,
    pivot: Pivot,
) {
    let mouse_world = coord::mouse_world_pos(ctx, camera);
    let snapped_position = snap_room_drag_position(mouse_world, grid_size, pivot);
    let preview = build_prefab_preview(ctx, prefab, asset_registry, sprite_manager);

    for item in &preview.items {
        let draw_pos = snapped_position + item.stamp_position;
        match item.visual {
            PrefabPreviewVisual::Sprite { sprite_id } => {
                let texture = sprite_manager.get_texture_from_id(ctx, sprite_id);
                ctx.draw_texture_ex(
                    texture,
                    draw_pos.x,
                    draw_pos.y,
                    Color::new(1.0, 1.0, 1.0, PREFAB_GHOST_OPACITY),
                    DrawTextureParams {
                        dest_size: Some(item.size),
                        ..Default::default()
                    },
                );
            }
            PrefabPreviewVisual::CurrentFrame {
                sprite_id,
                source,
                flip_x,
            } => {
                let texture = sprite_manager.get_texture_from_id(ctx, sprite_id);
                ctx.draw_texture_ex(
                    texture,
                    draw_pos.x,
                    draw_pos.y,
                    Color::new(1.0, 1.0, 1.0, PREFAB_GHOST_OPACITY),
                    DrawTextureParams {
                        dest_size: Some(item.size),
                        source: Some(source),
                        flip_x,
                        ..Default::default()
                    },
                );
            }
            PrefabPreviewVisual::Placeholder => {}
        }
    }

    let ghost_scale = grid_size / world_constants::DEFAULT_GRID_SIZE;
    for &(_palette_pos, stamp_pos, ref visual) in &preview.fallback_visuals {
        let draw_pos = snapped_position + stamp_pos * ghost_scale;
        let icon = match visual {
            EntityVisual::CameraIcon => camera_icon(),
            EntityVisual::PortalIcon => portal_icon(),
            EntityVisual::EntryIcon => entry_icon(),
            EntityVisual::ExitIcon => exit_icon(),
            EntityVisual::LightPlaceholder | EntityVisual::GlowPlaceholder => entity_icon(),
            EntityVisual::GenericPlaceholder => entity_icon(),
            EntityVisual::SpriteOrAnimation => continue,
        };
        let size = Vec2::splat(grid_size);
        ctx.draw_texture_ex(
            icon,
            draw_pos.x,
            draw_pos.y,
            Color::new(1.0, 1.0, 1.0, PREFAB_GHOST_OPACITY),
            DrawTextureParams {
                dest_size: Some(size),
                ..Default::default()
            },
        );
    }
}

/// Draw the outline of the collider for an entity if it has one.
pub fn draw_editor_collider(ctx: &mut WgpuContext, ecs: &Ecs, entity: Entity, grid_size: f32) {
    let visual_entity = resolve_visual_entity(ecs, entity);
    let Some(collider) = ecs.get_store::<Collider>().get(visual_entity) else {
        return;
    };
    let transform = match ecs.get_store::<Transform>().get(entity) {
        Some(t) => t,
        None => return,
    };

    let edit_active = is_collider_edit_active_for(visual_entity);
    let color = if edit_active {
        Color::new(0.0, 1.0, 1.0, 0.8)
    } else {
        Color::PINK
    };
    let thickness = outline_thickness(grid_size) * ENTITY_OUTLINE_SCALE;
    draw_collider(ctx, transform.position, collider, transform.pivot, color, thickness);

    if edit_active {
        let handles = compute_handles(transform.position, transform.pivot, collider, grid_size);
        for handle in &handles {
            ctx.draw_rectangle(
                handle.rect.x,
                handle.rect.y,
                handle.rect.w,
                handle.rect.h,
                Color::WHITE,
            );
            ctx.draw_rectangle_lines(
                handle.rect.x,
                handle.rect.y,
                handle.rect.w,
                handle.rect.h,
                0.5,
                Color::BLACK,
            );
        }
    }
}

/// Draw placeholder icons for entities that lack a visual component.
pub fn draw_entity_placeholders(
    ctx: &mut WgpuContext,
    ecs: &Ecs,
    sprite_manager: &mut SpriteManager,
    room_id: RoomId,
    layer: RoomLayer,
    grid_size: f32,
) {
    for &entity in ecs.entities_in_room_layer(room_id, layer) {
        ecs.assert_room_membership(room_id, entity);
        let Some(transform) = ecs.get_store::<Transform>().get(entity) else {
            continue;
        };
        let pos = transform.position;

        match resolve_entity_visual(ecs, entity) {
            EntityVisual::SpriteOrAnimation => {}
            EntityVisual::CameraIcon => {
                draw_camera_icon(ctx, pos, grid_size);
            }
            visual @ (EntityVisual::PortalIcon | EntityVisual::EntryIcon | EntityVisual::ExitIcon) => {
                let icon = match visual {
                    EntityVisual::PortalIcon => portal_icon(),
                    EntityVisual::EntryIcon => entry_icon(),
                    EntityVisual::ExitIcon => exit_icon(),
                    _ => unreachable!(),
                };
                let draw_pos = pivot_adjusted_position(
                    pos,
                    Vec2::splat(grid_size),
                    transform.pivot,
                );
                ctx.draw_texture_ex(
                    icon,
                    draw_pos.x,
                    draw_pos.y,
                    Color::new(1.0, 1.0, 1.0, PLACEHOLDER_OPACITY),
                    DrawTextureParams {
                        dest_size: Some(vec2(grid_size, grid_size)),
                        ..Default::default()
                    },
                );
            }
            EntityVisual::LightPlaceholder => {
                draw_light_placeholder(ctx, pos, grid_size);
            }
            EntityVisual::GlowPlaceholder => {
                draw_glow_placeholder(ctx, sprite_manager, ecs, entity, pos, grid_size);
            }
            EntityVisual::GenericPlaceholder => {
                let draw_pos = pivot_adjusted_position(
                    pos,
                    Vec2::splat(grid_size),
                    transform.pivot,
                );
                ctx.draw_texture_ex(
                    entity_icon(),
                    draw_pos.x,
                    draw_pos.y,
                    Color::new(1.0, 1.0, 1.0, PLACEHOLDER_OPACITY),
                    DrawTextureParams {
                        dest_size: Some(vec2(grid_size, grid_size)),
                        ..Default::default()
                    },
                );
            }
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

/// Returns true if the entity is a pure placeholder (no visual component).
pub fn is_pure_placeholder(ecs: &Ecs, entity: Entity) -> bool {
    matches!(
        resolve_entity_visual(ecs, entity),
        EntityVisual::CameraIcon | EntityVisual::LightPlaceholder
    )
}

/// Draw interaction guides for an entity.
pub fn draw_entity_interaction_guides(ctx: &mut WgpuContext, ecs: &Ecs, entity: Entity, grid_size: f32) {
    let Some(transform) = ecs.get_store::<Transform>().get(entity) else { return };
    let thickness = outline_thickness(grid_size) * ENTITY_OUTLINE_SCALE;
    let cx = transform.position.x;
    let cy = transform.position.y;

    if let Some(interactable) = ecs.get_store::<Interactable>().get(entity) {
        let violet = Color::new(0.75, 0.25, 1.0, 0.55);
        match interactable.shape() {
            InteractableShape::Circle => {
                let center = interactable.center_at(transform.position);
                ctx.draw_circle_lines(
                    center.x,
                    center.y,
                    interactable.radius,
                    thickness,
                    violet,
                );
            }
            InteractableShape::Rect => {
                let bounds = interactable.bounds_at(transform.position);
                ctx.draw_rectangle_lines(
                    bounds.x,
                    bounds.y,
                    bounds.w,
                    bounds.h,
                    thickness,
                    violet,
                );
            }
        }
    }

    let exit_range = ecs
        .get_store::<WorldExit>()
        .get(entity)
        .and_then(|e| match &e.trigger {
            WorldExitTrigger::OnProximity(r) => Some(*r),
            _ => None,
        });
    if let Some(range) = exit_range {
        let orange = Color::new(1.0, 0.55, 0.1, 0.55);
        ctx.draw_circle_lines(
            cx,
            cy,
            range,
            thickness,
            orange,
        );
    }
}

/// Draw interaction guides for each entity in the room.
pub fn draw_entity_interaction_guides_in_room(
    ctx: &mut WgpuContext,
    ecs: &Ecs,
    room_id: RoomId,
    layer: RoomLayer,
    grid_size: f32,
) {
    for &entity in ecs.entities_in_room_layer(room_id, layer) {
        ecs.assert_room_membership(room_id, entity);
        draw_entity_interaction_guides(ctx, ecs, entity, grid_size);
    }
}

/// Draw exit arrows for the active authored layer in the room.
pub fn draw_exit_placeholders(
    ctx: &mut WgpuContext,
    exits: &[Exit],
    room_position: Vec2,
    active_layer: RoomLayer,
    grid_size: f32,
) {
    for exit in exits.iter().filter(|exit| exit.layer == active_layer) {
        let position = exit.position * grid_size + room_position;
        draw_exit_arrow(ctx, position, exit.direction, exit.layer, active_layer, grid_size);
    }
}

/// Draw all camera viewports in a room.
pub fn draw_all_camera_viewports(
    ctx: &mut WgpuContext,
    editor_cam: &Camera2D,
    ecs: &Ecs,
    room_id: RoomId,
    layer: RoomLayer,
) {
    let cam_store = ecs.get_store::<RoomCamera>();
    let pos_store = ecs.get_store::<Transform>();

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

    for &entity in ecs.entities_in_room_layer(room_id, layer) {
        ecs.assert_room_membership(room_id, entity);

        let Some(room_cam) = cam_store.get(entity) else {
            continue;
        };

        let pos = match pos_store.get(entity) {
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
            with_theme(|t| t.accent),
        );
    }
}

/// Draw an exit arrow at the given position.
pub fn draw_exit_arrow(
    ctx: &mut WgpuContext,
    position: Vec2,
    direction: ExitDirection,
    layer: RoomLayer,
    active_layer: RoomLayer,
    grid_size: f32,
) {
    draw_exit_arrow_styled(
        ctx,
        position,
        direction,
        exit_arrow_style(layer, active_layer, false),
        grid_size,
    );
}

/// Draw an adjacent room's exit arrow.
pub fn draw_adjacent_exit_arrow(
    ctx: &mut WgpuContext,
    position: Vec2,
    direction: ExitDirection,
    layer: RoomLayer,
    active_layer: RoomLayer,
    grid_size: f32,
) {
    draw_exit_arrow_styled(
        ctx,
        position,
        direction,
        exit_arrow_style(layer, active_layer, true),
        grid_size,
    );
}

#[derive(Clone, Copy)]
struct ExitArrowStyle {
    color: Color,
}

fn exit_arrow_style(layer: RoomLayer, active_layer: RoomLayer, adjacent: bool) -> ExitArrowStyle {
    let base_color = match layer {
        RoomLayer::Front => Color::new(1.0, 0.75, 0.25, 1.0),
        RoomLayer::Back => Color::new(0.35, 0.82, 1.0, 1.0),
    };
    let alpha = if adjacent { 0.85 } else { 1.0 };

    ExitArrowStyle {
        color: base_color.with_alpha(if layer == active_layer { alpha } else { 0.0 }),
    }
}

fn draw_exit_arrow_styled(
    ctx: &mut WgpuContext,
    position: Vec2,
    direction: ExitDirection,
    style: ExitArrowStyle,
    grid_size: f32,
) {
    if style.color.a <= 0.0 {
        return;
    }

    let arrow_center = position + vec2(grid_size / 2.0, grid_size / 2.0);

    let offsets = match direction {
        ExitDirection::Up => [vec2(0.0, -1.0), vec2(-1.0, 1.0), vec2(1.0, 1.0)],
        ExitDirection::Down => [vec2(0.0, 1.0), vec2(-1.0, -1.0), vec2(1.0, -1.0)],
        ExitDirection::Left => [vec2(-1.0, 0.0), vec2(1.0, -1.0), vec2(1.0, 1.0)],
        ExitDirection::Right => [vec2(1.0, 0.0), vec2(-1.0, -1.0), vec2(-1.0, 1.0)],
    };
    let arrow_scale = grid_size / 3.0;

    ctx.draw_triangle(
        arrow_center + offsets[0] * arrow_scale,
        arrow_center + offsets[1] * arrow_scale,
        arrow_center + offsets[2] * arrow_scale,
        style.color,
    );
}
