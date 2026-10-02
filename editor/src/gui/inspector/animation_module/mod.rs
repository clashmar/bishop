use crate::editor_global::push_toast;
use crate::gui::gui_constants::*;
use bishop::prelude::*;
use engine_core::animation::{AseExportResult, ClipDef, ClipId, ClipState, JsonImportResult, VariantFolder, export_aseprite_folder, import_aseprite_metadata, import_variant_folder, resolve_json_path};
use engine_core::ecs::*;
use engine_core::game::{GameCtxMut};
use engine_core::storage::*;
use engine_core::ui::measure_text;
use ::widgets::*;
use std::{
    borrow::Cow,
    collections::HashSet,
    path::Path,
};
use ::widgets::constants::{colors, layout};
use strum::IntoEnumIterator;

mod clip_selector;
mod fields;
pub mod frame_edit;

use clip_selector::{draw_current_clip_dropdowns, fill_all_clip_ids};
use fields::{
    draw_fps_loop_and_mirrored, draw_frame_size_fields, draw_offset_fields,
    draw_spritesheet_dimension_fields,
};
use frame_edit::draw_frame_edit_controls;

// Width of a three‑digit numeric field
pub(super) const NUM_FIELD_W: f32 = 40.0;
pub(super) const LABEL_Y_OFFSET: f32 = 20.0;
pub(super) const LABEL_FONT_SIZE: f32 = layout::DEFAULT_FONT_SIZE_16;
pub(super) const COLON_GAP: f32 = 8.0;
pub(super) const COLUMN_GAP: f32 = 8.0;
pub(super) const INLINE_GROUP_GAP: f32 = 8.0;
const SECTION_SPACING: f32 = 10.0;
const BUTTON_ROW_HEIGHT: f32 = MARGIN;
const IMPORT_ROW_HEIGHT: f32 = MARGIN;
const ROW_ADVANCE: f32 = MARGIN + layout::WIDGET_PADDING;

#[derive(Default)]
pub struct AnimationModule {
    pending_rename: bool,
    rename_initial_value: String,
    has_clips: bool,
    variant_picker_id: WidgetId,
    select_dropdown_id: WidgetId,
    set_dropdown_id: WidgetId,
    rename_field_id: WidgetId,
    frame_x_id: WidgetId,
    frame_y_id: WidgetId,
    cols_id: WidgetId,
    rows_id: WidgetId,
    fps_id: WidgetId,
    offset_x_id: WidgetId,
    offset_y_id: WidgetId,
    frame_edit_toggle_id: WidgetId,
    prev_frame_id: WidgetId,
    next_frame_id: WidgetId,
}

impl InspectorModule for AnimationModule {
    fn undo_component_type(&self) -> Option<&'static str> {
        Some(<Animation>::TYPE_NAME)
    }

    fn visible(&self, ecs: &Ecs, entity: Entity) -> bool {
        ecs.get::<Animation>(entity).is_some()
    }

    fn removable(&self) -> bool {
        true
    }

    fn remove(&mut self, game_ctx: &mut GameCtxMut, entity: Entity) {
        Ecs::remove_component::<Animation>(game_ctx, entity);
        Ecs::remove_component::<CurrentFrame>(game_ctx, entity);
    }

    fn draw(
        &mut self,
        ctx: &mut WgpuContext,
        blocked: bool,
        rect: Rect,
        game_ctx: &mut GameCtxMut,
        entity: Entity,
    ) {
        let ecs = &mut game_ctx.ecs;

        let asset_registry = &mut game_ctx.asset_registry;
        let sprite_manager = &mut game_ctx.sprite_manager;

        let mut variant_changed = false;
        let mut clip_removed = false;
        let mut all_ids: Vec<ClipId> = vec![];
        fill_all_clip_ids(ecs, &mut all_ids);

        let animation = ecs
            .get_mut::<Animation>(entity)
            .expect("Animation must exist");

        let mut y = rect.y + layout::WIDGET_SPACING;
        let full_w = rect.w - 2.0 * layout::WIDGET_PADDING;

        // Track whether we have clips for dynamic height
        self.has_clips = !animation.clips.is_empty();

        // Button dimensions
        const ADD_LABEL: &str = "Add Clip";
        const REMOVE_LABEL: &str = "Remove Clip";
        let add_txt = measure_text(ctx, ADD_LABEL, layout::DEFAULT_FONT_SIZE_16);
        let remove_txt = measure_text(ctx, REMOVE_LABEL, layout::DEFAULT_FONT_SIZE_16);
        let btn_h = add_txt.height + 8.0;
        let add_btn_w = add_txt.width + 12.0;
        let remove_btn_w = remove_txt.width + 12.0;
        let btn_gap = 8.0;

        // Center both buttons together
        let total_btn_w = add_btn_w + btn_gap + remove_btn_w;
        let btn_start_x = rect.x + (rect.w - total_btn_w) / 2.0;

        let add_rect = Rect::new(btn_start_x, y, add_btn_w, btn_h);
        let remove_rect = Rect::new(btn_start_x + add_btn_w + btn_gap, y, remove_btn_w, btn_h);

        // Add clip button
        let mut clip_added = false;
        if Button::new(add_rect, ADD_LABEL)
            .suppressed(blocked)
            .show(ctx)
        {
            let new_id = if animation.clips.is_empty() {
                ClipId::Idle
            } else {
                let used: HashSet<_> = animation.clips.keys().cloned().collect();
                let next_builtin = ClipId::iter()
                    .filter(|id| !matches!(id, ClipId::New | ClipId::Custom(_)))
                    .find(|id| !used.contains(id));
                match next_builtin {
                    Some(id) => id.clone(),
                    None => ClipId::Custom(format!("New Clip {}", animation.clips.len() + 1)),
                }
            };
            animation.clips.insert(new_id.clone(), ClipDef::default());
            animation
                .states
                .insert(new_id.clone(), ClipState::default());
            animation.current = Some(new_id);
            clip_added = true;
            self.has_clips = true;
        }

        // Remove clip button
        let can_remove = animation.current.is_some();
        if Button::new(remove_rect, REMOVE_LABEL)
            .suppressed(blocked)
            .blocked(!can_remove)
            .show(ctx)
        {
            if let Some(current_id) = animation.current.take() {
                animation.clips.remove(&current_id);
                animation.states.remove(&current_id);

                // Select next available clip or clear
                animation.current = if animation.clips.is_empty() {
                    None
                } else if animation.clips.contains_key(&ClipId::Idle) {
                    Some(ClipId::Idle)
                } else {
                    animation.clips.keys().next().cloned()
                };

                self.has_clips = !animation.clips.is_empty();
                clip_removed = true;
            }
        }

        y += MARGIN + layout::WIDGET_PADDING;

        // Return if there is no current clip
        if animation.current.is_none() {
            return;
        }

        // Variant picker
        let has_variant = !animation.variant.0.as_os_str().is_empty();
        let variant_btn_w = full_w / 2.0;
        let sprite_btn = Rect::new(rect.x + layout::WIDGET_PADDING, y, variant_btn_w, MARGIN);

        if Button::new(
            sprite_btn,
            if has_variant {
                "Edit Variant"
            } else {
                "Choose Variant"
            },
        )
        .interaction_id(self.variant_picker_id)
        .suppressed(blocked)
        .show_native_dialog(ctx)
        {
            if let Some(path) = rfd::FileDialog::new()
                .set_directory(assets_folder())
                .pick_folder()
            {
                let normalized_path = sprite_manager.normalize_path(path);
                animation.variant = VariantFolder(normalized_path);
                variant_changed = true;
            }
        }

        let full_path = Path::new(&animation.variant.0);

        let variant_label = if has_variant {
            full_path
                .file_name()
                .map(|n| Cow::Owned(format!("/{}", n.to_string_lossy().into_owned())))
                .unwrap_or_else(|| Cow::Borrowed("/..."))
        } else {
            Cow::Borrowed("/...")
        };

        ctx.draw_text(
            &variant_label,
            sprite_btn.x + sprite_btn.w + layout::WIDGET_SPACING,
            y + LABEL_Y_OFFSET,
            layout::DEFAULT_FONT_SIZE_16,
            colors::DEFAULT_TEXT_COLOR,
        );

        y += MARGIN + layout::WIDGET_PADDING;

        // Calculate clip selector dropdown here
        let clip_dropdown_rect = Rect::new(rect.x + layout::WIDGET_PADDING, y, full_w, BTN_HEIGHT);

        y += MARGIN + layout::WIDGET_PADDING;

        let Some(current_clip_id) = animation.current.clone() else {
            return;
        };

        if let Some(clip) = animation.clips.get(&current_clip_id) {
            draw_frame_edit_controls(
                ctx,
                self,
                entity,
                animation,
                clip,
                Rect::new(rect.x + layout::WIDGET_PADDING, y, full_w, MARGIN),
                blocked,
            );
            y += ROW_ADVANCE;
        }

        // Edit the currently selected clip
        if let Some(clip) = animation.clips.get_mut(&current_clip_id) {
            // Frame size
            draw_frame_size_fields(ctx, self, y, rect, clip, blocked);
            y += MARGIN + layout::WIDGET_PADDING;

            // Columns / rows
            draw_spritesheet_dimension_fields(ctx, self, y, rect, clip, blocked);
            y += MARGIN + layout::WIDGET_PADDING;

            // FPS / Loop / Mirrored toggles
            draw_fps_loop_and_mirrored(ctx, self, y, rect, clip, blocked);
            y += ROW_ADVANCE;

            // Optional offset
            draw_offset_fields(ctx, self, y, rect, clip, blocked);
            y += ROW_ADVANCE;

            // Import buttons at the bottom: "Import: [JSON] [Variant]"
            const IMPORT_LABEL: &str = "Import:";
            const JSON_LABEL: &str = "JSON";
            const VARIANT_LABEL: &str = "Variant";

            let import_label_w = measure_text(ctx, IMPORT_LABEL, LABEL_FONT_SIZE).width + COLON_GAP;
            let json_btn_w =
                measure_text(ctx, JSON_LABEL, layout::DEFAULT_FONT_SIZE_16).width + 16.0;
            let variant_btn_w =
                measure_text(ctx, VARIANT_LABEL, layout::DEFAULT_FONT_SIZE_16).width + 16.0;
            let btn_gap = 8.0;

            let start_x = rect.x + layout::WIDGET_PADDING;

            ctx.draw_text(
                IMPORT_LABEL,
                start_x,
                y + LABEL_Y_OFFSET,
                LABEL_FONT_SIZE,
                colors::DEFAULT_TEXT_COLOR,
            );

            let import_json_btn = Rect::new(start_x + import_label_w, y, json_btn_w, MARGIN);
            let import_variant_btn = Rect::new(
                import_json_btn.x + json_btn_w + btn_gap,
                y,
                variant_btn_w,
                MARGIN,
            );

            // Import JSON button - imports metadata for the current clip only
            if Button::new(import_json_btn, JSON_LABEL)
                .suppressed(blocked)
                .blocked(!has_variant)
                .show(ctx)
            {
                let json_path = resolve_json_path(&animation.variant, &current_clip_id);
                match import_aseprite_metadata(&json_path) {
                    JsonImportResult::Success(imported) => {
                        clip.frame_size = imported.frame_size;
                        clip.cols = imported.cols;
                        clip.rows = imported.rows;
                        clip.fps = imported.fps;
                        clip.frame_durations = imported.frame_durations;
                        clip.offset = imported.offset;
                        clip.mirrored = imported.mirrored;
                        push_toast("Import successful", 2.0);
                    }
                    JsonImportResult::NotFound => {
                        push_toast(format!("JSON not found: {}", json_path.display()), 3.0);
                    }
                    JsonImportResult::Error(msg) => {
                        push_toast(format!("Import error: {}", msg), 3.0);
                    }
                }
            }

            // Import Variant button - one-click full import from Aseprite files
            if Button::new(import_variant_btn, VARIANT_LABEL)
                .suppressed(blocked)
                .blocked(!has_variant)
                .show(ctx)
            {
                let full_path = assets_folder().join(&animation.variant.0);

                // Export all Aseprite files to PNG + JSON
                match export_aseprite_folder(&full_path) {
                    AseExportResult::Success => {}
                    AseExportResult::AsepriteNotFound => {
                        push_toast("Aseprite not found in PATH", 3.0);
                        return;
                    }
                    AseExportResult::ExportFailed { file, error } => {
                        push_toast(format!("Export failed: {}: {}", file, error), 3.0);
                        return;
                    }
                }

                // Import all JSON files (skips malformed JSON, not fatal)
                match import_variant_folder(&full_path) {
                    Ok(result) => {
                        // Clear existing clips and add new ones
                        animation.clips = result.clips;
                        animation.states.clear();
                        for id in animation.clips.keys() {
                            animation.states.insert(id.clone(), ClipState::default());
                        }
                        animation.current = animation.clips.keys().next().cloned();

                        let count = animation.clips.len();
                        let msg = if result.skipped.is_empty() {
                            format!("Imported {} clips", count)
                        } else {
                            format!(
                                "Imported {} clips ({} skipped)",
                                count,
                                result.skipped.len()
                            )
                        };
                        push_toast(msg, 2.0);

                        // Refresh sprite cache after importing
                        let has_variant_folder = !animation.variant.0.as_os_str().is_empty();
                        if has_variant_folder {
                            animation.refresh_sprite_cache(ctx, asset_registry, sprite_manager);
                            animation.init_runtime();
                        }
                    }
                    Err(e) => {
                        push_toast(format!("Import failed: {}", e), 3.0);
                    }
                }
            }
        }

        let clip_renamed = draw_current_clip_dropdowns(
            ctx,
            self,
            clip_dropdown_rect,
            entity,
            animation,
            all_ids,
            blocked,
        );

        // Refresh sprite cache when variant changes or a new clip is added (only if variant is set)
        let has_variant = !animation.variant.0.as_os_str().is_empty();
        if variant_changed || clip_added || clip_removed || clip_renamed {
            if has_variant {
                animation.refresh_sprite_cache(ctx, asset_registry, sprite_manager);
            } else if clip_removed || clip_renamed {
                animation.clear_sprite_cache();
            }
        }
    }

    fn body_layout(&self) -> InspectorBodyLayout {
        if self.has_clips {
            return InspectorBodyLayout::new()
                .top_padding(layout::WIDGET_SPACING)
                .rows(8, SECTION_SPACING)
                .gap(SECTION_SPACING)
                .block(IMPORT_ROW_HEIGHT);
        }

        InspectorBodyLayout::new()
            .top_padding(layout::WIDGET_SPACING)
            .bottom_gutter(layout::WIDGET_PADDING)
            .block(BUTTON_ROW_HEIGHT)
    }
}


inventory::submit! {
    ModuleFactoryEntry {
        type_name: <Animation>::TYPE_NAME,
        title: <Animation>::TYPE_NAME,
        factory: || {
            Box::new(
                CollapsibleComponentModule::new(
                    crate::gui::inspector::animation_module::AnimationModule::default()
                )
                .with_title(<Animation>::TYPE_NAME)
            )
        },
        allowed_for: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn populated_animation_height_matches_drawn_sections() {
        let module = AnimationModule {
            has_clips: true,
            ..Default::default()
        };

        assert_eq!(module.body_layout().height(), 370.0);
    }
}

