mod groups;
mod layout;
mod preview;

use self::groups::*;
use self::layout::body_layout;
pub use self::preview::clear_active_audio_preview;
use self::preview::*;
use crate::storage::sound_preset_storage::*;
use bishop::prelude::*;
use engine_core::prelude::*;

const TOP_PADDING: f32 = 10.0;
const SPACING: f32 = 5.0;
const SECTION_GAP: f32 = 12.0;
const EDIT_SECTION_SPACING: f32 = 9.0;
const ROW_HEIGHT: f32 = DEFAULT_FIELD_HEIGHT;
const LABEL_W: f32 = 80.0;
const VOLUME_LABEL_DECIMALS: usize = 2;
pub(super) const PREVIEW_HANDLE: u64 = 0x4544_4954_4F52_5052;
pub(super) const PREVIEW_TIMEOUT_SECONDS: f32 = 5.0;

/// Editor inspector module for the `AudioSource` component.
#[derive(Default)]
pub struct AudioSourceModule {
    select_dropdown_id: WidgetId,
    assign_dropdown_id: WidgetId,
    rename_field_id: WidgetId,
    preset_action_dropdown_id: WidgetId,
    volume_id: WidgetId,
    pitch_id: WidgetId,
    volume_var_id: WidgetId,
    warning: Option<Toast>,
    pending_rename_target: Option<SoundGroupId>,
    rename_initial_value: String,
    has_groups: bool,
    has_preset_actions: bool,
    sounds_len: usize,
}

impl InspectorModule for AudioSourceModule {
    fn undo_component_type(&self) -> Option<&'static str> {
        Some(AudioSource::TYPE_NAME)
    }

    fn visible(&self, ecs: &Ecs, entity: Entity) -> bool {
        ecs.get::<AudioSource>(entity).is_some()
    }

    fn removable(&self) -> bool {
        true
    }

    fn remove(&mut self, game_ctx: &mut GameCtxMut, entity: Entity) {
        clear_active_audio_preview();
        Ecs::remove_component::<AudioSource>(game_ctx, entity);
    }

    fn body_layout(&self) -> InspectorBodyLayout {
        body_layout(
            self.has_groups,
            self.pending_rename_target.is_some(),
            self.has_preset_actions,
            self.sounds_len,
        )
    }

    fn draw(
        &mut self,
        ctx: &mut WgpuContext,
        blocked: bool,
        rect: Rect,
        game_ctx: &mut GameCtxMut,
        entity: Entity,
    ) {
        tick_active_audio_preview(ctx.get_frame_time());

        let library = current_sound_preset_library();
        let mut pending_sync_all: Option<(String, AudioGroup)> = None;
        let mut pending_link_rename: Option<(String, String)> = None;
        let mut warning_message: Option<String> = None;

        {
            let Some(source) = game_ctx.ecs.get_mut::<AudioSource>(entity) else {
                return;
            };

            ensure_selected_group(source);
            self.has_groups = !source.groups.is_empty();
            self.has_preset_actions = false;

            let mut y = rect.y + TOP_PADDING;
            let x = rect.x + WIDGET_PADDING;
            let w = rect.w - 2.0 * WIDGET_PADDING;

            draw_group_dropdowns(
                ctx,
                blocked,
                Rect::new(x, y, w, ROW_HEIGHT),
                self,
                source,
                &library,
                &mut warning_message,
                &mut pending_sync_all,
            );
            y += ROW_HEIGHT + SPACING;

            if self
                .pending_rename_target
                .as_ref()
                .is_some_and(|group_id| !source.groups.contains_key(group_id))
            {
                self.pending_rename_target = None;
                text_input_reset(self.rename_field_id);
            }

            if self.pending_rename_target.is_some() {
                if let Some(message) = draw_rename_field(
                    ctx,
                    blocked,
                    Rect::new(x, y, w, ROW_HEIGHT),
                    self,
                    source,
                    &mut pending_link_rename,
                ) {
                    warning_message = Some(message);
                }
                y += ROW_HEIGHT + SPACING;
            }

            let Some(current_group_id) = source.current.clone() else {
                clear_active_audio_preview();
                self.has_preset_actions = false;
                self.sounds_len = 0;
                self.update_warning(ctx, warning_message);
                return;
            };

            let Some(group) = source.groups.get(&current_group_id) else {
                clear_active_audio_preview();
                self.has_preset_actions = false;
                self.sounds_len = 0;
                self.update_warning(ctx, Some("Current sound group is missing".to_string()));
                return;
            };

            let status_text = preset_status_text(group, &library);
            let preset_actions = preset_actions_for_group(&current_group_id, group, &library);
            self.has_preset_actions = !preset_actions.is_empty();
            if !preset_actions.is_empty() {
                if let Some(action) = Dropdown::new(
                    self.preset_action_dropdown_id,
                    Rect::new(x, y, w, ROW_HEIGHT),
                    "Preset Actions",
                    &preset_actions,
                    PresetAction::label,
                )
                .fixed_width()
                .right_aligned()
                .blocked(blocked)
                .show(ctx)
                {
                    if let Some(message) =
                        handle_preset_action(source, action, &mut pending_sync_all)
                    {
                        warning_message = Some(message);
                    }
                }
                y += ROW_HEIGHT + SPACING;
            }

            y += SECTION_GAP;

            let half_w = ((w - SPACING) * 0.5).max(0.0);
            let status_rect = Rect::new(x, y, half_w, ROW_HEIGHT);
            let add_rect = Rect::new(x + half_w + SPACING, y, half_w, ROW_HEIGHT);
            if Button::new(add_rect, "Add Sound")
                .blocked(blocked)
                .show(ctx)
            {
                #[cfg(not(target_arch = "wasm32"))]
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("Audio", &["wav"])
                    .set_directory(engine_core::storage::path_utils::audio_folder())
                    .pick_file()
                {
                    let base = engine_core::storage::path_utils::audio_folder();
                    let relative = path.strip_prefix(&base).unwrap_or(&path);
                    let sound_id = relative
                        .with_extension("")
                        .to_string_lossy()
                        .replace('\\', "/");

                    apply_source_edit(source, |source| {
                        if let Some(group) = source.groups.get_mut(&current_group_id) {
                            group.sounds.push(sound_id);
                        }
                    });
                }
            }

            let truncated_status = truncate_to_width(
                ctx,
                &status_text,
                status_rect.w.max(0.0),
                DEFAULT_FONT_SIZE_16,
            );
            ctx.draw_text(
                &truncated_status,
                status_rect.x,
                status_rect.y + 20.0,
                DEFAULT_FONT_SIZE_16,
                FIELD_TEXT_COLOR,
            );
            y += ROW_HEIGHT + EDIT_SECTION_SPACING;

            let sounds = source
                .groups
                .get(&current_group_id)
                .map(|group| group.sounds.clone())
                .unwrap_or_default();
            sync_active_preview(entity, &current_group_id, &sounds);
            self.sounds_len = sounds.len();

            let preview_group = source.groups.get(&current_group_id).cloned();
            let mut preview_request: Option<PreviewRequest> = None;
            let mut remove_idx: Option<usize> = None;
            for (index, sound) in sounds.iter().enumerate() {
                let remove_btn_w = ROW_HEIGHT;
                let preview_btn_w = 52.0;
                let label_rect = Rect::new(
                    x,
                    y,
                    w - preview_btn_w - remove_btn_w - SPACING * 2.0,
                    ROW_HEIGHT,
                );
                let preview_rect = Rect::new(
                    x + w - preview_btn_w - remove_btn_w - SPACING,
                    y,
                    preview_btn_w,
                    ROW_HEIGHT,
                );
                let remove_rect = Rect::new(x + w - remove_btn_w, y, remove_btn_w, ROW_HEIGHT);

                ctx.draw_text(
                    sound,
                    label_rect.x,
                    label_rect.y + 20.0,
                    DEFAULT_FONT_SIZE_16,
                    FIELD_TEXT_COLOR,
                );

                if Button::new(preview_rect, "Test")
                    .blocked(blocked || preview_group.is_none())
                    .show(ctx)
                {
                    preview_request = Some(PreviewRequest::new(index, sound.clone()));
                }

                if Button::new(remove_rect, "x").blocked(blocked).show(ctx) {
                    remove_idx = Some(index);
                }
                y += ROW_HEIGHT + EDIT_SECTION_SPACING;
            }

            if let Some(next_preview) = preview_request {
                if let Some(group) = preview_group.as_ref() {
                    apply_preview_request(entity, &current_group_id, Some(next_preview), group);
                } else {
                    clear_active_audio_preview();
                }
            }

            if let Some(index) = remove_idx {
                apply_source_edit(source, |source| {
                    if let Some(group) = source.groups.get_mut(&current_group_id) {
                        group.sounds.remove(index);
                    }
                });
            }

            if let Some(group) = source.groups.get_mut(&current_group_id) {
                ctx.draw_text(
                    "Volume:",
                    x,
                    y + 20.0,
                    DEFAULT_FONT_SIZE_16,
                    FIELD_TEXT_COLOR,
                );
                let volume_label = format_volume_label(group.volume);
                let volume_measure = measure_text(ctx, &volume_label, DEFAULT_FONT_SIZE_16);
                let value_x = x + LABEL_W + SPACING;
                ctx.draw_text(
                    &volume_label,
                    value_x,
                    y + 20.0,
                    DEFAULT_FONT_SIZE_16,
                    FIELD_TEXT_COLOR,
                );
                let slider_rect = Rect::new(
                    value_x + volume_measure.width + SPACING * 2.0,
                    y,
                    w - LABEL_W - volume_measure.width - SPACING * 4.0,
                    ROW_HEIGHT,
                );
                let (new_vol, state) =
                    gui_slider(ctx, self.volume_id, slider_rect, 0.0, 1.0, group.volume);
                if !blocked && !matches!(state, SliderState::Unchanged) {
                    group.volume = new_vol;
                }
                y += ROW_HEIGHT + EDIT_SECTION_SPACING;

                ctx.draw_text(
                    "Pitch Var:",
                    x,
                    y + 20.0,
                    DEFAULT_FONT_SIZE_16,
                    FIELD_TEXT_COLOR,
                );
                let slider_rect =
                    Rect::new(x + LABEL_W + SPACING, y, w - LABEL_W - SPACING, ROW_HEIGHT);
                let (new_pitch, state) = gui_slider(
                    ctx,
                    self.pitch_id,
                    slider_rect,
                    0.0,
                    1.0,
                    group.pitch_variation,
                );
                if !blocked && !matches!(state, SliderState::Unchanged) {
                    group.pitch_variation = new_pitch;
                }
                y += ROW_HEIGHT + EDIT_SECTION_SPACING;

                ctx.draw_text(
                    "Vol Var:",
                    x,
                    y + 20.0,
                    DEFAULT_FONT_SIZE_16,
                    FIELD_TEXT_COLOR,
                );
                let slider_rect =
                    Rect::new(x + LABEL_W + SPACING, y, w - LABEL_W - SPACING, ROW_HEIGHT);
                let (new_vol_var, state) = gui_slider(
                    ctx,
                    self.volume_var_id,
                    slider_rect,
                    0.0,
                    1.0,
                    group.volume_variation,
                );
                if !blocked && !matches!(state, SliderState::Unchanged) {
                    group.volume_variation = new_vol_var;
                }
                y += ROW_HEIGHT + EDIT_SECTION_SPACING;

                ctx.draw_text(
                    "Looping:",
                    x,
                    y + 20.0,
                    DEFAULT_FONT_SIZE_16,
                    FIELD_TEXT_COLOR,
                );
                let cb_rect = Rect::new(
                    x + LABEL_W + SPACING,
                    y + (ROW_HEIGHT - DEFAULT_CHECKBOX_DIMS) / 2.0,
                    DEFAULT_CHECKBOX_DIMS,
                    DEFAULT_CHECKBOX_DIMS,
                );
                if !blocked {
                    gui_checkbox(ctx, cb_rect, &mut group.looping);
                }
            }
        }

        if let Some((preset_name, preset)) = pending_sync_all {
            sync_linked_groups_from_preset(game_ctx.ecs, &preset_name, &preset);
        }
        if let Some((old_preset_name, new_preset_name)) = pending_link_rename {
            rename_preset_links_in_ecs(game_ctx.ecs, &old_preset_name, &new_preset_name);
        }

        self.update_warning(ctx, warning_message);
    }
}

impl AudioSourceModule {
    fn update_warning(&mut self, ctx: &mut WgpuContext, warning_message: Option<String>) {
        if let Some(message) = warning_message {
            self.warning = Some(Toast::new(message, 2.5));
        }

        if let Some(toast) = &mut self.warning {
            toast.update(ctx);
            if !toast.active {
                self.warning = None;
            }
        }
    }
}

fn format_volume_label(volume: f32) -> String {
    format!("{volume:.VOLUME_LABEL_DECIMALS$}x")
}

inventory::submit! {
    ModuleFactoryEntry {
        title: <AudioSource>::TYPE_NAME,
        factory: || {
            Box::new(
                CollapsibleModule::new(
                    crate::gui::inspector::audio_source_module::AudioSourceModule::default()
                )
                .with_title("Audio Source")
            )
        },
    }
}

#[cfg(test)]
mod tests;
