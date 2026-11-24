use crate::editor_global::push_toast;
use bishop::prelude::{Rect, WgpuContext};
use engine_core::animation::{
    ClipDef, ClipId, extract_named_aseprite_slice_bounds, resolve_json_path,
};
use engine_core::ecs::{Animation, Entity, Glow, GlowSource, GLOW_SLICE_NAME};
use engine_core::game::GameCtxMut;
use std::fs;
use widgets::{Button, Widget, WidgetId};

use super::ROW_H;
use super::super::aseprite_slice_import::format_aseprite_slice_import_toast;

const IMPORT_ANIMATION_SLICES_LABEL: &str = "Import Animation Slices";

#[derive(Default)]
pub(super) struct GlowSyncUi {
    pub import_slices_id: WidgetId,
}

pub(super) fn should_show_aseprite_slice_import(glow: &Glow) -> bool {
    glow.source == GlowSource::AsepriteSlice
}

pub(super) fn draw_sync_controls(
    ctx: &mut WgpuContext,
    ui: &mut GlowSyncUi,
    blocked: bool,
    rect: Rect,
    game_ctx: &mut GameCtxMut,
    entity: Entity,
) {
    let Some(glow) = game_ctx.ecs.get::<Glow>(entity) else {
        return;
    };
    if !should_show_aseprite_slice_import(glow) {
        return;
    }

    let has_animation = game_ctx.ecs.get::<Animation>(entity).is_some();
    if !Button::new(
        Rect::new(rect.x, rect.y, rect.w, ROW_H),
        IMPORT_ANIMATION_SLICES_LABEL,
    )
        .interaction_id(ui.import_slices_id)
        .blocked(blocked || !has_animation)
        .show(ctx)
    {
        return;
    }

    if let Some(new_glow) = import_all_clips_for_entity(game_ctx, entity) {
        if let Some(glow) = game_ctx.ecs.get_mut::<Glow>(entity) {
            *glow = new_glow;
        }
    } else {
        push_toast("Cannot import Aseprite glow slices: missing animation data", 3.0);
    }
}

fn import_all_clips_for_entity(game_ctx: &mut GameCtxMut, entity: Entity) -> Option<Glow> {
    let mut synced = game_ctx.ecs.get::<Glow>(entity)?.clone();
    let animation = game_ctx.ecs.get::<Animation>(entity)?;
    let mut warning_count = 0usize;
    let mut imported_count = 0usize;
    let mut failure_count = 0usize;

    for (clip_id, clip) in &animation.clips {
        let json_path = resolve_json_path(&animation.variant, clip_id);
        let result = match fs::read_to_string(json_path) {
            Ok(json) => update_clip_slice_bounds(&mut synced, clip_id.clone(), clip, Ok(&json)),
            Err(error) => Err(error.to_string()),
        };

        match result {
            Ok((imported, warnings)) => {
                imported_count += imported;
                warning_count += warnings;
            }
            Err(_) => failure_count += 1,
        }
    }

    let (_, message, duration) = format_aseprite_slice_import_toast(
        imported_count,
        warning_count,
        failure_count,
        "glow slice frame(s)",
        "Aseprite glow slice import failed",
        "No Aseprite glow slices found",
    );
    push_toast(message, duration);

    Some(synced)
}

fn update_clip_slice_bounds(
    glow: &mut Glow,
    clip_id: ClipId,
    clip: &ClipDef,
    json: Result<&str, String>,
) -> Result<(usize, usize), String> {
    let json = json?;
    let imported = extract_named_aseprite_slice_bounds(json, clip, GLOW_SLICE_NAME)
        .map_err(|error| format!("{error:?}"))?;
    let imported_count = imported.frames.len();
    let warning_count = imported.warnings.len();
    glow.replace_slice_bounds_for_clip(clip_id, imported.frames);
    Ok((imported_count, warning_count))
}

#[cfg(test)]
mod tests {
    use super::{should_show_aseprite_slice_import, update_clip_slice_bounds};
    use bishop::prelude::vec2;
    use engine_core::animation::{AsepriteSliceBounds, ClipDef, ClipId};
    use engine_core::ecs::{Glow, GlowSource, GLOW_SLICE_NAME};
    use std::collections::BTreeMap;

    fn clip() -> ClipDef {
        ClipDef {
            cols: 2,
            rows: 2,
            frame_size: vec2(100.0, 80.0),
            ..Default::default()
        }
    }

    fn slice_json(slices: &str) -> String {
        format!(
            r##"{{
                "frames": {{}},
                "meta": {{
                    "image": "Idle.png",
                    "size": {{ "w": 200, "h": 160 }},
                    "slices": [{}]
                }}
            }}"##,
            slices,
        )
    }

    fn old_bounds() -> AsepriteSliceBounds {
        AsepriteSliceBounds {
            x: 1.0,
            y: 2.0,
            w: 3.0,
            h: 4.0,
        }
    }

    #[test]
    fn import_control_is_visible_only_for_aseprite_slice_source() {
        let mut glow = Glow::default();
        assert!(!should_show_aseprite_slice_import(&glow));

        glow.source = GlowSource::AsepriteSlice;
        assert!(should_show_aseprite_slice_import(&glow));
    }

    #[test]
    fn glow_slice_sync_replaces_bounds_for_the_imported_clip() {
        let mut glow = Glow::default();
        glow.replace_slice_bounds_for_clip(
            ClipId::Idle,
            BTreeMap::from([(0, old_bounds())]),
        );
        let glow_slice = format!(
            r##"{{
                "name": "{}",
                "keys": [
                    {{ "frame": 1, "bounds": {{ "x": 10, "y": 12, "w": 16, "h": 8 }} }}
                ]
            }}"##,
            GLOW_SLICE_NAME,
        );
        let json = slice_json(&glow_slice);

        let (imported_count, warning_count) = update_clip_slice_bounds(
            &mut glow,
            ClipId::Idle,
            &clip(),
            Ok(&json),
        )
        .expect("valid glow slice JSON should import");

        assert!(glow.slice_bounds_for_frame(&ClipId::Idle, 0).is_none());
        assert_eq!(
            glow.slice_bounds_for_frame(&ClipId::Idle, 1),
            Some(AsepriteSliceBounds {
                x: 10.0,
                y: 12.0,
                w: 16.0,
                h: 8.0,
            }),
        );
        assert_eq!(imported_count, 1);
        assert_eq!(warning_count, 0);
    }

    #[test]
    fn glow_slice_sync_clears_bounds_when_import_succeeds_without_slice() {
        let mut glow = Glow::default();
        glow.replace_slice_bounds_for_clip(
            ClipId::Idle,
            BTreeMap::from([(0, old_bounds())]),
        );
        let json = slice_json(
            r##"{
                "name": "selection",
                "keys": [
                    { "frame": 0, "bounds": { "x": 1, "y": 2, "w": 3, "h": 4 } }
                ]
            }"##,
        );

        let (imported_count, warning_count) = update_clip_slice_bounds(
            &mut glow,
            ClipId::Idle,
            &clip(),
            Ok(&json),
        )
        .expect("valid JSON without a glow slice should import successfully");

        assert!(glow.slice_bounds_for_frame(&ClipId::Idle, 0).is_none());
        assert_eq!(imported_count, 0);
        assert_eq!(warning_count, 0);
    }

    #[test]
    fn glow_slice_sync_preserves_bounds_when_json_is_missing_or_invalid() {
        let mut glow = Glow::default();
        glow.replace_slice_bounds_for_clip(
            ClipId::Idle,
            BTreeMap::from([(0, old_bounds())]),
        );

        let missing = update_clip_slice_bounds(&mut glow, ClipId::Idle, &clip(), Err("missing".into()));
        let invalid = update_clip_slice_bounds(&mut glow, ClipId::Idle, &clip(), Ok("not json"));

        assert!(missing.is_err());
        assert!(invalid.is_err());
        assert_eq!(glow.slice_bounds_for_frame(&ClipId::Idle, 0), Some(old_bounds()));
    }
}
