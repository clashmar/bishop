use std::collections::HashSet;

use bishop::prelude::*;
use engine_core::assets::*;
use engine_core::constants::world as world_constants;
use engine_core::ecs::*;
use engine_core::rendering::{
    draw_collider, outline_thickness, pivot_adjusted_position, resolve_visual_entity,
    ENTITY_OUTLINE_SCALE,
};
use engine_core::theme::with_theme;
use engine_core::worlds::*;

use crate::app::control::camera_controller::EditorCameraController;
use crate::editor_assets::assets::{camera_icon, entity_icon, entry_icon, exit_icon, portal_icon};
use crate::gui::inspector::collider_module::edit::{compute_handles, is_collider_edit_active_for};
use crate::gui::inspector::interactable_module::edit::{
    compute_handles as compute_interactable_handles,
    is_interactable_edit_active_for,
};
use crate::room::bounds_edit::draw_handles;
use crate::room::prefab_preview::{build_prefab_preview, PrefabPreviewVisual};
use crate::room::room_editor::*;
use crate::room::selection::{entity_selection_rect, snap_room_drag_position};
use crate::shared::entity_icon::{
    draw_camera_icon, draw_glow_placeholder, draw_light_placeholder, resolve_entity_visual,
    EntityVisual, PLACEHOLDER_OPACITY,
};
use crate::world::coord;

const PREFAB_GHOST_OPACITY: f32 = 0.55;

impl RoomEditor {
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
        draw_handles(ctx, &handles);
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
        let edit_active = is_interactable_edit_active_for(entity);
        let violet = if edit_active {
            Color::new(0.85, 0.35, 1.0, 0.85)
        } else {
            Color::new(0.75, 0.25, 1.0, 0.55)
        };
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
        if edit_active {
            let handles = compute_interactable_handles(transform.position, interactable, grid_size);
            draw_handles(ctx, &handles);
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

/// Draw interaction guides for each unselected entity in the room.
pub fn draw_entity_interaction_guides_in_room(
    ctx: &mut WgpuContext,
    ecs: &Ecs,
    room_id: RoomId,
    layer: RoomLayer,
    grid_size: f32,
    selected_entities: &HashSet<Entity>,
) {
    for &entity in ecs.entities_in_room_layer(room_id, layer) {
        ecs.assert_room_membership(room_id, entity);
        if selected_entities.contains(&entity) {
            continue;
        }
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
