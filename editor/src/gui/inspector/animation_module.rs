// editor/src/gui/inspector/animation_module.rs
use crate::gui::gui_constants::*;
use bishop::prelude::*;
use engine_core::prelude::*;
use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    path::Path,
};
use strum::IntoEnumIterator;

// Width of a three‑digit numeric field
const NUM_FIELD_W: f32 = 40.0;
const LABEL_Y_OFFSET: f32 = 20.0;
const LABEL_FONT_SIZE: f32 = DEFAULT_FONT_SIZE_16;
const COLON_GAP: f32 = 10.0;
const FIELD_GAP: f32 = 20.0;
const SECTION_SPACING: f32 = 10.0;
const BUTTON_ROW_HEIGHT: f32 = MARGIN;
const IMPORT_ROW_HEIGHT: f32 = MARGIN;

#[derive(Default)]
pub struct AnimationModule {
    pending_rename: bool,
    rename_initial_value: String,
    warning: Option<Toast>,
    has_clips: bool,
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

        let asset_manager = &mut game_ctx.asset_manager;

        let mut variant_changed = false;
        let mut all_ids: Vec<ClipId> = vec![];
        fill_all_clip_ids(ecs, &mut all_ids);

        let animation = ecs
            .get_mut::<Animation>(entity)
            .expect("Animation must exist");

        let mut y = rect.y + WIDGET_SPACING;
        let full_w = rect.w - 2.0 * WIDGET_PADDING;

        // Track whether we have clips for dynamic height
        self.has_clips = !animation.clips.is_empty();

        // Button dimensions
        const ADD_LABEL: &str = "Add Clip";
        const REMOVE_LABEL: &str = "Remove Clip";
        let add_txt = measure_text(ctx, ADD_LABEL, DEFAULT_FONT_SIZE_16);
        let remove_txt = measure_text(ctx, REMOVE_LABEL, DEFAULT_FONT_SIZE_16);
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
        if Button::new(add_rect, ADD_LABEL).blocked(blocked).show(ctx) {
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
            .blocked(blocked || !can_remove)
            .show(ctx)
        {
            if let Some(current_id) = animation.current.take() {
                animation.clips.remove(&current_id);
                animation.states.remove(&current_id);
                animation.sprite_cache.remove(&current_id);

                // Select next available clip or clear
                animation.current = if animation.clips.is_empty() {
                    None
                } else if animation.clips.contains_key(&ClipId::Idle) {
                    Some(ClipId::Idle)
                } else {
                    Some(animation.clips.keys().next().unwrap().clone())
                };

                self.has_clips = !animation.clips.is_empty();
            }
        }

        y += MARGIN + WIDGET_PADDING;

        // Return if there is no current clip
        if animation.current.is_none() {
            return;
        }

        // Variant picker
        let has_variant = !animation.variant.0.as_os_str().is_empty();
        let variant_btn_w = full_w / 2.0;
        let sprite_btn = Rect::new(rect.x + WIDGET_PADDING, y, variant_btn_w, MARGIN);

        if Button::new(
            sprite_btn,
            if has_variant {
                "Edit Variant"
            } else {
                "Choose Variant"
            },
        )
        .blocked(blocked)
        .show(ctx)
        {
            if let Some(path) = rfd::FileDialog::new().pick_folder() {
                let normalized_path = asset_manager.normalize_path(path);
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
            sprite_btn.x + sprite_btn.w + WIDGET_SPACING,
            y + LABEL_Y_OFFSET,
            DEFAULT_FONT_SIZE_16,
            FIELD_TEXT_COLOR,
        );

        y += MARGIN + WIDGET_PADDING;

        // Calculate clip selector dropdown here
        let clip_dropdown_rect = Rect::new(rect.x + WIDGET_PADDING, y, full_w, BTN_HEIGHT);

        y += MARGIN + WIDGET_PADDING;

        let current_clip_id = animation.current.clone().unwrap();

        // Edit the currently selected clip
        if let Some(clip) = animation.clips.get_mut(&current_clip_id) {
            // Frame size
            draw_frame_size_fields(ctx, self, y, rect, clip, blocked);
            y += MARGIN + WIDGET_PADDING;

            // Columns / rows
            draw_spritesheet_dimension_fields(ctx, self, y, rect, clip, blocked);
            y += MARGIN + WIDGET_PADDING;

            // FPS / Loop / Mirrored toggles
            draw_fps_loop_and_mirrored(ctx, self, y, rect, clip, blocked);
            y += MARGIN + WIDGET_PADDING;

            // Optional offset
            draw_offset_fields(ctx, self, y, rect, clip, blocked);
            y += MARGIN + WIDGET_PADDING;

            // Import buttons at the bottom: "Import: [JSON] [Variant]"
            const IMPORT_LABEL: &str = "Import:";
            const JSON_LABEL: &str = "JSON";
            const VARIANT_LABEL: &str = "Variant";

            let import_label_w = measure_text(ctx, IMPORT_LABEL, LABEL_FONT_SIZE).width + COLON_GAP;
            let json_btn_w = measure_text(ctx, JSON_LABEL, DEFAULT_FONT_SIZE_16).width + 16.0;
            let variant_btn_w = measure_text(ctx, VARIANT_LABEL, DEFAULT_FONT_SIZE_16).width + 16.0;
            let btn_gap = 8.0;

            let start_x = rect.x + WIDGET_PADDING;

            ctx.draw_text(
                IMPORT_LABEL,
                start_x,
                y + LABEL_Y_OFFSET,
                LABEL_FONT_SIZE,
                FIELD_TEXT_COLOR,
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
                .blocked(blocked || !has_variant)
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
                        self.warning = Some(Toast::new("Import successful".to_string(), 2.0));
                    }
                    JsonImportResult::NotFound => {
                        self.warning = Some(Toast::new(
                            format!("JSON not found: {}", json_path.display()),
                            3.0,
                        ));
                    }
                    JsonImportResult::Error(msg) => {
                        self.warning = Some(Toast::new(format!("Import error: {}", msg), 3.0));
                    }
                }
            }

            // Import Variant button - one-click full import from Aseprite files
            if Button::new(import_variant_btn, VARIANT_LABEL)
                .blocked(blocked || !has_variant)
                .show(ctx)
            {
                let full_path = assets_folder().join(&animation.variant.0);

                // Export all Aseprite files to PNG + JSON
                match export_aseprite_folder(&full_path) {
                    AseExportResult::Success => {}
                    AseExportResult::AsepriteNotFound => {
                        self.warning =
                            Some(Toast::new("Aseprite not found in PATH".to_string(), 3.0));
                        return;
                    }
                    AseExportResult::ExportFailed { file, error } => {
                        self.warning = Some(Toast::new(
                            format!("Export failed: {}: {}", file, error),
                            3.0,
                        ));
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
                        self.warning = Some(Toast::new(msg, 2.0));

                        // Refresh sprite cache after importing
                        let has_variant_folder = !animation.variant.0.as_os_str().is_empty();
                        if has_variant_folder {
                            animation.refresh_sprite_cache(ctx, asset_manager);
                            animation.init_runtime();
                        }
                    }
                    Err(e) => {
                        self.warning = Some(Toast::new(format!("Import failed: {}", e), 3.0));
                    }
                }
            }
        }

        draw_current_clip_dropdowns(ctx, self, clip_dropdown_rect, animation, all_ids, blocked);

        if let Some(toast) = &mut self.warning {
            toast.update(ctx);
            if !toast.active {
                self.warning = None;
            }
        }

        // Refresh sprite cache when variant changes or a new clip is added (only if variant is set)
        let has_variant = !animation.variant.0.as_os_str().is_empty();
        if (variant_changed || clip_added) && has_variant {
            animation.refresh_sprite_cache(ctx, asset_manager);
        }
    }

    fn body_layout(&self) -> InspectorBodyLayout {
        if self.has_clips {
            return InspectorBodyLayout::new()
                .top_padding(WIDGET_SPACING)
                .rows(7, SECTION_SPACING)
                .gap(SECTION_SPACING)
                .block(IMPORT_ROW_HEIGHT);
        }

        InspectorBodyLayout::new()
            .top_padding(WIDGET_SPACING)
            .bottom_gutter(WIDGET_PADDING)
            .block(BUTTON_ROW_HEIGHT)
    }
}

pub fn draw_current_clip_dropdowns(
    ctx: &mut WgpuContext,
    module: &mut AnimationModule,
    rect: Rect,
    animation: &mut Animation,
    all_ids: Vec<ClipId>,
    blocked: bool,
) {
    let current_id = animation.current.as_ref().unwrap();
    let clip_label = format!("{current_id}");
    let width = rect.w / 2.0 - WIDGET_SPACING;

    // Select clip
    let select_rect = Rect::new(rect.x, rect.y, width, rect.h);

    if let Some(selected) = Dropdown::new(
        module.select_dropdown_id,
        select_rect,
        &clip_label,
        &existing_clip_ids(&animation.clips),
        |id| id.ui_label(),
    )
    .blocked(blocked)
    .show(ctx)
    {
        animation.set_clip(&selected);
        return;
    }

    // Edit the ClipId of the current clip
    let right_rect = Rect::new((select_rect.x + rect.w) - (width), rect.y, width, rect.h);

    // Show the type selector
    let type_label = "Set Type";

    let chosen = Dropdown::new(
        module.set_dropdown_id,
        right_rect,
        type_label,
        &all_ids,
        |id| id.ui_label(),
    )
    .blocked(blocked)
    .show(ctx);

    if let Some(chosen) = chosen {
        match chosen {
            // For now always open the rename field
            ClipId::New => {
                module.pending_rename = true;
                module.rename_initial_value.clear();
                return;
            }
            ClipId::Custom(name) => {
                module.pending_rename = true;
                module.rename_initial_value = name.clone();
                return;
            }
            // Any other enum variant
            other => {
                // Prevent duplicate concrete types on the same entity
                if animation.clips.contains_key(&other)
                    && Some(&other) != animation.current.as_ref()
                {
                    module.warning = Some(Toast::new("Enity already has this animation.", 2.0));
                } else {
                    reset_current_clip_id(animation, other);
                    module.pending_rename = false;
                }
                return;
            }
        }
    }

    // Render the rename text field while the flag is true
    if module.pending_rename {
        // Position directly under the right‑hand dropdown
        let input_rect = Rect::new(
            right_rect.x,
            right_rect.y + right_rect.h + 4.0,
            right_rect.w,
            INPUT_HEIGHT,
        );

        const CLAMP: usize = 12;

        // The field starts empty each time we open it
        let (entered, focused) = TextInput::new(
            module.rename_field_id,
            input_rect,
            &module.rename_initial_value,
        )
        .max_len(CLAMP)
        .focused(true)
        .blocked(blocked)
        .show(ctx);

        // Check if enter is pressed first
        if ctx.is_key_pressed(KeyCode::Enter) {
            let new_id = ClipId::Custom(entered.trim().to_string());
            reset_current_clip_id(animation, new_id);
            module.pending_rename = false;
            text_input_reset(module.rename_field_id);
        } else if !focused {
            text_input_reset(module.rename_field_id);
            module.pending_rename = false;
        }
    }
}

pub fn draw_frame_size_fields(
    ctx: &mut WgpuContext,
    module: &mut AnimationModule,
    y: f32,
    rect: Rect,
    clip: &mut ClipDef,
    blocked: bool,
) {
    const LABELS: [&str; 2] = ["Frame X:", "Frame Y:"];
    let (lbl_x, inp_x, lbl_y, inp_y) = layout_pair(ctx, y, rect, LABELS);

    // Render the two labels
    ctx.draw_text(
        LABELS[0],
        lbl_x.x,
        lbl_x.y,
        LABEL_FONT_SIZE,
        FIELD_TEXT_COLOR,
    );
    ctx.draw_text(
        LABELS[1],
        lbl_y.x,
        lbl_y.y,
        LABEL_FONT_SIZE,
        FIELD_TEXT_COLOR,
    );

    // Numeric inputs
    clip.frame_size.x = NumberInput::new(module.frame_x_id, inp_x, clip.frame_size.x)
        .blocked(blocked)
        .show(ctx);
    clip.frame_size.y = NumberInput::new(module.frame_y_id, inp_y, clip.frame_size.y)
        .blocked(blocked)
        .show(ctx);
}

pub fn draw_spritesheet_dimension_fields(
    ctx: &mut WgpuContext,
    module: &mut AnimationModule,
    y: f32,
    rect: Rect,
    clip: &mut ClipDef,
    blocked: bool,
) {
    const LABELS: [&str; 2] = ["Cols:", "Rows:"];
    let (lbl_c, inp_c, lbl_r, inp_r) = layout_pair(ctx, y, rect, LABELS);

    ctx.draw_text(
        LABELS[0],
        lbl_c.x,
        lbl_c.y,
        LABEL_FONT_SIZE,
        FIELD_TEXT_COLOR,
    );
    ctx.draw_text(
        LABELS[1],
        lbl_r.x,
        lbl_r.y,
        LABEL_FONT_SIZE,
        FIELD_TEXT_COLOR,
    );

    clip.cols = NumberInput::new(module.cols_id, inp_c, clip.cols as f32)
        .blocked(blocked)
        .show(ctx) as usize;
    clip.rows = NumberInput::new(module.rows_id, inp_r, clip.rows as f32)
        .blocked(blocked)
        .show(ctx) as usize;
}

pub fn draw_fps_loop_and_mirrored(
    ctx: &mut WgpuContext,
    module: &mut AnimationModule,
    y: f32,
    rect: Rect,
    clip: &mut ClipDef,
    blocked: bool,
) {
    const LABELS: [&str; 2] = ["FPS:", "Loop:"];
    let (lbl_fps, inp_fps, lbl_loop, mut inp_loop) = layout_pair(ctx, y, rect, LABELS);
    inp_loop.w = CHECKBOX_SIZE;
    inp_loop.h = CHECKBOX_SIZE;
    inp_loop.y += 5.;

    ctx.draw_text(
        LABELS[0],
        lbl_fps.x,
        lbl_fps.y,
        LABEL_FONT_SIZE,
        FIELD_TEXT_COLOR,
    );
    ctx.draw_text(
        LABELS[1],
        lbl_loop.x,
        lbl_loop.y,
        LABEL_FONT_SIZE,
        FIELD_TEXT_COLOR,
    );

    clip.fps = NumberInput::new(module.fps_id, inp_fps, clip.fps)
        .blocked(blocked)
        .show(ctx);
    gui_checkbox(ctx, inp_loop, &mut clip.looping);

    // Mirrored checkbox
    let mirrored_label = "Mirror:";
    let mirrored_label_w = measure_text(ctx, mirrored_label, LABEL_FONT_SIZE).width + COLON_GAP;
    let mirrored_lbl_x = inp_loop.x + inp_loop.w + FIELD_GAP;
    ctx.draw_text(
        mirrored_label,
        mirrored_lbl_x,
        lbl_loop.y,
        LABEL_FONT_SIZE,
        FIELD_TEXT_COLOR,
    );

    let inp_mirrored = Rect::new(
        mirrored_lbl_x + mirrored_label_w,
        inp_loop.y,
        CHECKBOX_SIZE,
        CHECKBOX_SIZE,
    );
    gui_checkbox(ctx, inp_mirrored, &mut clip.mirrored);
}

pub fn draw_offset_fields(
    ctx: &mut WgpuContext,
    module: &mut AnimationModule,
    y: f32,
    rect: Rect,
    clip: &mut ClipDef,
    blocked: bool,
) {
    const LABELS: [&str; 2] = ["Offset X:", "Offset Y:"];
    let (lbl_x, inp_x, lbl_y, inp_y) = layout_pair(ctx, y, rect, LABELS);

    ctx.draw_text(
        LABELS[0],
        lbl_x.x,
        lbl_x.y,
        LABEL_FONT_SIZE,
        FIELD_TEXT_COLOR,
    );
    ctx.draw_text(
        LABELS[1],
        lbl_y.x,
        lbl_y.y,
        LABEL_FONT_SIZE,
        FIELD_TEXT_COLOR,
    );

    clip.offset.x = NumberInput::new(module.offset_x_id, inp_x, clip.offset.x)
        .blocked(blocked)
        .show(ctx);
    clip.offset.y = NumberInput::new(module.offset_y_id, inp_y, clip.offset.y)
        .blocked(blocked)
        .show(ctx);
}

/// Returns every ClipId that has a concrete Clip stored in the map.
fn existing_clip_ids(clips: &HashMap<ClipId, ClipDef>) -> Vec<ClipId> {
    clips.keys().cloned().collect()
}

/// Adds every possible `ClipId` to the supplied Vec.
pub fn fill_all_clip_ids(ecs: &Ecs, out: &mut Vec<ClipId>) {
    // Built‑in IDs
    let mut ids: Vec<ClipId> = ClipId::iter()
        .filter(|id| !matches!(id, ClipId::New | ClipId::Custom(_)))
        .collect();

    // Gather every custom type
    let mut custom_names = HashSet::new();
    for animation in ecs.get_store::<Animation>().data.values() {
        for clip_id in animation.clips.keys() {
            if let ClipId::Custom(name) = clip_id {
                custom_names.insert(name.clone());
            }
        }
    }

    // Sort the custom values
    let mut custom_ids: Vec<ClipId> = custom_names.into_iter().map(ClipId::Custom).collect();

    custom_ids.sort_by_key(|id| id.ui_label());

    // Assemble the final list with New at the end
    ids.extend(custom_ids);
    ids.push(ClipId::New);
    *out = ids;
}

/// Helper that moves the currently selected clip under a new `ClipId`.
fn reset_current_clip_id(animation: &mut Animation, new_id: ClipId) {
    let old_id = animation.current.take().unwrap();

    // Take the old clip out of the map
    if let Some(old_clip) = animation.clips.remove(&old_id) {
        // Insert it under the new key
        animation.clips.insert(new_id.clone(), old_clip);
    }

    // Move the runtime state as well
    if let Some(state) = animation.states.remove(&old_id) {
        animation.states.insert(new_id.clone(), state);
    }

    // Finally make the renamed clip the active one
    animation.current = Some(new_id);
}

/// Returns the two label rects and the two input rects for a horizontal pair of fields.
fn layout_pair(
    ctx: &mut WgpuContext,
    y: f32,
    rect: Rect,
    labels: [&'static str; 2],
) -> (Rect, Rect, Rect, Rect) {
    // Width of each label
    let width1 = measure_text(ctx, labels[0], LABEL_FONT_SIZE).width + COLON_GAP;
    let width2 = measure_text(ctx, labels[1], LABEL_FONT_SIZE).width + COLON_GAP;

    // First label
    let label1 = Rect::new(
        rect.x + WIDGET_PADDING,
        y + LABEL_Y_OFFSET,
        width1,
        INPUT_HEIGHT,
    );

    // First input
    let input0 = Rect::new(label1.x + width1, y, NUM_FIELD_W, INPUT_HEIGHT);

    // Second label
    let label2 = Rect::new(
        input0.x + NUM_FIELD_W + FIELD_GAP,
        y + LABEL_Y_OFFSET,
        width2,
        INPUT_HEIGHT,
    );

    // Second input
    let input1 = Rect::new(label2.x + width2, y, NUM_FIELD_W, INPUT_HEIGHT);

    (label1, input0, label2, input1)
}

inventory::submit! {
    ModuleFactoryEntry {
        title: <engine_core::animation::animation_clip::Animation>::TYPE_NAME,
        factory: || {
            Box::new(
                CollapsibleModule::new(
                    crate::gui::inspector::animation_module::AnimationModule::default()
                )
                .with_title(<engine_core::animation::animation_clip::Animation>::TYPE_NAME)
            )
        },
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

        assert_eq!(module.body_layout().height(), 330.0);
    }
}
