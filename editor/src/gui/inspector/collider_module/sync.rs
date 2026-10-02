use crate::editor_global::push_toast;
use bishop::prelude::{Rect, WgpuContext};
use engine_core::animation::{ClipId, extract_aseprite_collider_slices, resolve_json_path};
use engine_core::ecs::{Animation, Collider, ColliderData, ColliderFrameKey, Ecs, Entity, Pivot, Transform};
use engine_core::game::GameCtxMut;
use std::collections::BTreeMap;
use std::fs;
use widgets::{Button, WidgetId};

use super::ROW_H;

const IMPORT_ASEPRITE_SLICES_LABEL: &str = "Import Aseprite Slices";

#[derive(Default)]
pub(super) struct ColliderSyncUi {
    pub import_slices_id: WidgetId,
}

pub(super) fn should_show_aseprite_slice_import(ecs: &Ecs, entity: Entity) -> bool {
    ecs.get::<Collider>(entity).is_some() && ecs.get::<Animation>(entity).is_some()
}

pub(super) fn aseprite_slice_import_label() -> &'static str {
    IMPORT_ASEPRITE_SLICES_LABEL
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum AsepriteSliceImportToast {
    Success,
    Warning,
    Failure,
}

pub(super) fn aseprite_slice_import_toast(
    imported_count: usize,
    warning_count: usize,
    failure_count: usize,
) -> (AsepriteSliceImportToast, String, f32) {
    if imported_count > 0 {
        if warning_count > 0 || failure_count > 0 {
            return (
                AsepriteSliceImportToast::Warning,
                format!(
                    "Imported {imported_count} collider slice(s), {warning_count} warning(s), {failure_count} failure(s)"
                ),
                3.0,
            );
        }

        return (
            AsepriteSliceImportToast::Success,
            format!("Imported {imported_count} collider slice(s)"),
            2.0,
        );
    }

    if failure_count > 0 {
        return (
            AsepriteSliceImportToast::Failure,
            format!("Aseprite slice import failed for {failure_count} clip(s)"),
            3.0,
        );
    }

    (
        AsepriteSliceImportToast::Warning,
        "No Aseprite collider slices found".to_string(),
        3.0,
    )
}

pub(super) fn apply_imported_frame_data(
    collider: &Collider,
    clip_id: ClipId,
    frames: &BTreeMap<ColliderFrameKey, ColliderData>,
) -> Collider {
    let mut synced = collider.clone();
    for (frame, data) in frames {
        synced.set_frame_data(clip_id.clone(), *frame, *data);
    }
    synced
}

pub(super) fn draw_sync_controls(
    ctx: &mut WgpuContext,
    ui: &mut ColliderSyncUi,
    blocked: bool,
    rect: Rect,
    game_ctx: &mut GameCtxMut,
    entity: Entity,
) {
    if !should_show_aseprite_slice_import(game_ctx.ecs, entity) {
        return;
    }

    if Button::new(
        Rect::new(rect.x, rect.y, rect.w, ROW_H),
        aseprite_slice_import_label(),
    )
    .interaction_id(ui.import_slices_id)
    .suppressed(blocked)
    .show(ctx)
    {
        if let Some(new_collider) = import_all_clips_for_entity(game_ctx, entity) {
            replace_entity_collider(game_ctx, entity, new_collider);
        } else {
            push_toast("Cannot import Aseprite slices: missing animation data", 3.0);
        }
    }
}

fn import_all_clips_for_entity(game_ctx: &mut GameCtxMut, entity: Entity) -> Option<Collider> {
    let pivot = selected_transform_pivot(game_ctx.ecs, entity).unwrap_or_default();
    let mut synced = game_ctx.ecs.get::<Collider>(entity)?.clone();
    let animation = game_ctx.ecs.get::<Animation>(entity)?;
    let mut warning_count = 0usize;
    let mut imported_count = 0usize;
    let mut failure_count = 0usize;

    for (clip_id, clip) in &animation.clips {
        let json_path = resolve_json_path(&animation.variant, clip_id);
        let Ok(json) = fs::read_to_string(&json_path) else {
            failure_count += 1;
            continue;
        };
        let Ok(imported) = extract_aseprite_collider_slices(&json, clip, pivot) else {
            failure_count += 1;
            continue;
        };
        warning_count += imported.warnings.len();
        imported_count += imported.frames.len();
        synced = apply_imported_frame_data(&synced, clip_id.clone(), &imported.frames);
    }

    let (_, message, duration) = aseprite_slice_import_toast(
        imported_count,
        warning_count,
        failure_count,
    );
    push_toast(message, duration);

    Some(synced)
}

fn selected_transform_pivot(ecs: &Ecs, entity: Entity) -> Option<Pivot> {
    ecs.get::<Transform>(entity).map(|transform| transform.pivot)
}

fn replace_entity_collider(game_ctx: &mut GameCtxMut, entity: Entity, new_collider: Collider) {
    if let Some(collider) = game_ctx.ecs.get_mut::<Collider>(entity) {
        *collider = new_collider;
    }
}
