use std::collections::HashSet;
use std::path::Path;
use std::path::PathBuf;

use crate::editor_global::push_toast;
use crate::gui::inspector::audio_source_module::clear_active_audio_preview;
use crate::gui::inspector::audio_source_module::groups::*;
use crate::gui::inspector::audio_source_module::layout::body_layout;
use crate::gui::inspector::audio_source_module::preview::*;
use crate::storage::sound_presets::*;
use bishop::prelude::*;
use engine_core::assets::*;
use engine_core::constants::paths;
use engine_core::ecs::*;
use engine_core::game::GameCtxMut;
use engine_core::storage::*;
use engine_core::ui::measure_text;
use ::widgets::*;
use ::widgets::constants::colors;
use ::widgets::constants::layout as widget_layout;

pub(crate) const TOP_PADDING: f32 = 10.0;
pub(crate) const SPACING: f32 = 5.0;
pub(crate) const SECTION_GAP: f32 = 12.0;
pub(crate) const EDIT_SECTION_SPACING: f32 = 9.0;
pub(crate) const ROW_HEIGHT: f32 = widget_layout::DEFAULT_FIELD_HEIGHT;
pub(crate) const PREVIEW_HANDLE: u64 = 0x4544_4954_4F52_5052;
pub(crate) const PREVIEW_TIMEOUT_SECONDS: f32 = 5.0;
const LABEL_W: f32 = 80.0;
const VOLUME_LABEL_DECIMALS: usize = 2;

/// Shared audio authoring state and rendering, reusable by entity, room, and world wrappers.
#[derive(Default)]
pub(crate) struct AudioSourceModuleCore {
    pub(crate) select_dropdown_id: WidgetId,
    pub(crate) assign_dropdown_id: WidgetId,
    pub(crate) load_preset_dropdown_id: WidgetId,
    pub(crate) rename_field_id: WidgetId,
    pub(crate) preset_action_dropdown_id: WidgetId,
    pub(crate) add_sound_button_id: WidgetId,
    pub(crate) volume_id: WidgetId,
    pub(crate) pitch_id: WidgetId,
    pub(crate) volume_var_id: WidgetId,
    pub(crate) stop_behavior_dropdown_id: WidgetId,
    pub(crate) fade_duration_id: WidgetId,
    pub(crate) pending_rename_target: Option<SoundGroupId>,
    pub(crate) rename_initial_value: String,
    pub(crate) show_preset_picker: bool,
    pub(crate) preset_picker_rect: Option<Rect>,
    pub(crate) has_groups: bool,
    pub(crate) has_preset_actions: bool,
    pub(crate) has_fade_duration: bool,
    pub(crate) sounds_len: usize,
}

impl AudioSourceModuleCore {
    /// Component type name used by the framework's undo machinery.
    pub(crate) fn undo_component_type() -> Option<&'static str> {
        Some(AudioSource::TYPE_NAME)
    }

    /// Layout for the audio authoring UI, based on cached state.
    pub(crate) fn body_layout(&self) -> InspectorBodyLayout {
        body_layout(
            self.has_groups,
            self.pending_rename_target.is_some(),
            self.has_preset_actions,
            self.has_fade_duration,
            self.sounds_len,
        )
    }

    /// Renders the full audio authoring UI for the given entity.
    pub(crate) fn draw(
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
            self.preset_picker_rect = None;
            self.has_groups = !source.groups.is_empty();
            self.has_preset_actions = false;

            let mut y = rect.y + TOP_PADDING;
            let x = rect.x + widget_layout::WIDGET_PADDING;
            let w = rect.w - 2.0 * widget_layout::WIDGET_PADDING;

            if let Some(message) = draw_group_dropdowns(
                ctx,
                blocked,
                Rect::new(x, y, w, ROW_HEIGHT),
                self,
                source,
            ) {
                warning_message = Some(message);
            }
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
                if let Some(message) = render_preset_picker(
                    ctx,
                    blocked,
                    self,
                    source,
                    &library,
                    &mut pending_sync_all,
                ) {
                    warning_message = Some(message);
                }
                if let Some(msg) = warning_message {
                    push_toast(msg, 2.5);
                }
                return;
            };

            let Some(group) = source.groups.get(&current_group_id) else {
                clear_active_audio_preview();
                self.has_preset_actions = false;
                self.sounds_len = 0;
                push_toast("Current sound group is missing", 2.5);
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
                .suppressed(blocked)
                .show(ctx)
                {
                    if let Some(message) = handle_preset_action(
                        source,
                        action,
                        &mut pending_sync_all,
                    ) {
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
                .interaction_id(self.add_sound_button_id)
                .suppressed(blocked)
                .show_native_dialog(ctx)
            {
                #[cfg(not(target_arch = "wasm32"))]
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("Audio", &["wav"])
                    .set_directory(audio_folder())
                    .pick_file()
                {
                    let base = audio_folder();
                    let relative = path.strip_prefix(&base).unwrap_or(&path);
                    match register_sound_id(game_ctx.asset_registry, relative) {
                        Ok(sound_id) => {
                            apply_source_edit(source, |source| {
                                if let Some(group) = source.groups.get_mut(&current_group_id) {
                                    group.sounds.push(sound_id);
                                }
                            });
                        }
                        Err(message) => warning_message = Some(message),
                    }
                }
            }

            let truncated_status = truncate_to_width(
                ctx,
                &status_text,
                status_rect.w.max(0.0),
                widget_layout::DEFAULT_FONT_SIZE_16,
            );
            ctx.draw_text(
                &truncated_status,
                status_rect.x,
                status_rect.y + 20.0,
                widget_layout::DEFAULT_FONT_SIZE_16,
                colors::DEFAULT_TEXT_COLOR,
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
                    &sound_label(game_ctx.asset_registry, *sound),
                    label_rect.x,
                    label_rect.y + 20.0,
                    widget_layout::DEFAULT_FONT_SIZE_16,
                    colors::DEFAULT_TEXT_COLOR,
                );

                if Button::new(preview_rect, "Test")
                    .suppressed(blocked)
                    .blocked(preview_group.is_none())
                    .show(ctx)
                {
                    preview_request = Some(PreviewRequest::new(index, *sound));
                }

                if Button::new(remove_rect, "x").suppressed(blocked).show(ctx) {
                    remove_idx = Some(index);
                }
                y += ROW_HEIGHT + EDIT_SECTION_SPACING;
            }

            if let Some(next_preview) = preview_request {
                if let Some(group) = preview_group.as_ref() {
                    apply_preview_request(
                        entity,
                        &current_group_id,
                        Some(next_preview),
                        group,
                        game_ctx.asset_registry,
                    );
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
                    widget_layout::DEFAULT_FONT_SIZE_16,
                    colors::DEFAULT_TEXT_COLOR,
                );
                let volume_label = format_volume_label(group.volume);
                let volume_measure =
                    measure_text(ctx, &volume_label, widget_layout::DEFAULT_FONT_SIZE_16);
                let value_x = x + LABEL_W + SPACING;
                ctx.draw_text(
                    &volume_label,
                    value_x,
                    y + 20.0,
                    widget_layout::DEFAULT_FONT_SIZE_16,
                    colors::DEFAULT_TEXT_COLOR,
                );
                let slider_rect = Rect::new(
                    value_x + volume_measure.width + SPACING * 2.0,
                    y,
                    w - LABEL_W - volume_measure.width - SPACING * 4.0,
                    ROW_HEIGHT,
                );
                let (new_vol, state) =
                    Slider::new(self.volume_id, slider_rect, 0.0, 1.0, group.volume).show(ctx);
                if !blocked && !matches!(state, SliderState::Unchanged) {
                    group.volume = new_vol;
                }
                y += ROW_HEIGHT + EDIT_SECTION_SPACING;

                ctx.draw_text(
                    "Pitch Var:",
                    x,
                    y + 20.0,
                    widget_layout::DEFAULT_FONT_SIZE_16,
                    colors::DEFAULT_TEXT_COLOR,
                );
                let slider_rect =
                    Rect::new(x + LABEL_W + SPACING, y, w - LABEL_W - SPACING, ROW_HEIGHT);
                let (new_pitch, state) =
                    Slider::new(self.pitch_id, slider_rect, 0.0, 1.0, group.pitch_variation)
                        .show(ctx);
                if !blocked && !matches!(state, SliderState::Unchanged) {
                    group.pitch_variation = new_pitch;
                }
                y += ROW_HEIGHT + EDIT_SECTION_SPACING;

                ctx.draw_text(
                    "Vol Var:",
                    x,
                    y + 20.0,
                    widget_layout::DEFAULT_FONT_SIZE_16,
                    colors::DEFAULT_TEXT_COLOR,
                );
                let slider_rect =
                    Rect::new(x + LABEL_W + SPACING, y, w - LABEL_W - SPACING, ROW_HEIGHT);
                let (new_vol_var, state) = Slider::new(
                    self.volume_var_id,
                    slider_rect,
                    0.0,
                    1.0,
                    group.volume_variation,
                )
                .show(ctx);
                if !blocked && !matches!(state, SliderState::Unchanged) {
                    group.volume_variation = new_vol_var;
                }
                y += ROW_HEIGHT + EDIT_SECTION_SPACING;

                ctx.draw_text(
                    "Looping:",
                    x,
                    y + 20.0,
                    widget_layout::DEFAULT_FONT_SIZE_16,
                    colors::DEFAULT_TEXT_COLOR,
                );
                let cb_rect = Rect::new(
                    x + LABEL_W + SPACING,
                    y + (ROW_HEIGHT - widget_layout::DEFAULT_CHECKBOX_DIMS) / 2.0,
                    widget_layout::DEFAULT_CHECKBOX_DIMS,
                    widget_layout::DEFAULT_CHECKBOX_DIMS,
                );
                Checkbox::new(cb_rect, &mut group.looping)
                    .blocked(blocked)
                    .show(ctx);
                y += ROW_HEIGHT + EDIT_SECTION_SPACING;

                // Auto-play checkbox
                let mut auto_play = matches!(group.trigger, AudioTrigger::OnOwnerActivate);
                let cb_rect = Rect::new(
                    x + LABEL_W + SPACING,
                    y + (ROW_HEIGHT - widget_layout::DEFAULT_CHECKBOX_DIMS) / 2.0,
                    widget_layout::DEFAULT_CHECKBOX_DIMS,
                    widget_layout::DEFAULT_CHECKBOX_DIMS,
                );
                Checkbox::new(cb_rect, &mut auto_play)
                    .blocked(blocked)
                    .show(ctx);
                group.trigger = if auto_play {
                    AudioTrigger::OnOwnerActivate
                } else {
                    AudioTrigger::Manual
                };
                ctx.draw_text(
                    "Autoplay:",
                    x,
                    y + 20.0,
                    widget_layout::DEFAULT_FONT_SIZE_16,
                    colors::DEFAULT_TEXT_COLOR,
                );
                y += ROW_HEIGHT + EDIT_SECTION_SPACING;

                // Stop behavior dropdown
                let stop_options: Vec<String> = vec!["Immediate".to_string(), "Fade Out".to_string()];
                let current_stop_label = stop_behavior_label(&group.stop_behavior);
                if let Some(selected) = Dropdown::new(
                    self.stop_behavior_dropdown_id,
                    Rect::new(x + LABEL_W + SPACING, y, w - LABEL_W - SPACING, ROW_HEIGHT),
                    &current_stop_label,
                    &stop_options,
                    |label| label.clone(),
                )
                .right_aligned()
                .suppressed(blocked)
                .show(ctx)
                {
                    if selected == "Fade Out" {
                        group.stop_behavior = AudioStopBehavior::FadeOut { duration: 0.5 };
                    } else {
                        group.stop_behavior = AudioStopBehavior::Immediate;
                    }
                }
                ctx.draw_text(
                    "Stop:",
                    x,
                    y + 20.0,
                    widget_layout::DEFAULT_FONT_SIZE_16,
                    colors::DEFAULT_TEXT_COLOR,
                );
                y += ROW_HEIGHT + EDIT_SECTION_SPACING;

                // Fade duration slider (only when FadeOut)
                self.has_fade_duration = matches!(group.stop_behavior, AudioStopBehavior::FadeOut { .. });
                if let AudioStopBehavior::FadeOut { duration } = &mut group.stop_behavior {
                    let fade_label = format!("Fade: {:.2}s", duration);
                    let fade_measure = measure_text(ctx, &fade_label, widget_layout::DEFAULT_FONT_SIZE_16);
                    let value_x = x + LABEL_W + SPACING;
                    ctx.draw_text(
                        &fade_label,
                        value_x,
                        y + 20.0,
                        widget_layout::DEFAULT_FONT_SIZE_16,
                        colors::DEFAULT_TEXT_COLOR,
                    );
                    let slider_rect = Rect::new(
                        value_x + fade_measure.width + SPACING * 2.0,
                        y,
                        w - LABEL_W - fade_measure.width - SPACING * 4.0,
                        ROW_HEIGHT,
                    );
                    let (new_dur, state) =
                        Slider::new(self.fade_duration_id, slider_rect, 0.0, 5.0, *duration).show(ctx);
                    if !blocked && !matches!(state, SliderState::Unchanged) {
                        *duration = new_dur;
                    }
                }
            }

            if let Some(message) = render_preset_picker(
                ctx,
                blocked,
                self,
                source,
                &library,
                &mut pending_sync_all,
            ) {
                warning_message = Some(message);
            }
        }

        if let Some((preset_name, preset)) = pending_sync_all {
            sync_linked_groups_from_preset(game_ctx.ecs, &preset_name, &preset);
        }
        if let Some((old_preset_name, new_preset_name)) = pending_link_rename {
            rename_preset_links_in_ecs(game_ctx.ecs, &old_preset_name, &new_preset_name);
        }

        if let Some(msg) = warning_message {
            push_toast(msg, 2.5);
        }
    }
}

fn next_sound_id(asset_registry: &AssetRegistry) -> SoundId {
    let used = asset_registry
        .records()
        .keys()
        .filter_map(|key| match key {
            AssetKey::Sound(sound_id) if sound_id.0 != 0 => Some(sound_id.0),
            _ => None,
        })
        .collect::<HashSet<_>>();

    let mut candidate = 1usize;
    while used.contains(&candidate) {
        candidate += 1;
    }
    SoundId(candidate)
}

fn register_sound_id(
    asset_registry: &mut AssetRegistry,
    relative_wav_path: &Path,
) -> Result<SoundId, String> {
    let registry_path = PathBuf::from(paths::AUDIO_FOLDER).join(relative_wav_path);
    let sound_id = match asset_registry.key_for_path(&registry_path) {
        Some(AssetKey::Sound(sound_id)) => sound_id,
        _ => next_sound_id(asset_registry),
    };

    asset_registry
        .register_asset_relative_path(sound_id, relative_wav_path)
        .map_err(|error| error.to_string())?;
    Ok(sound_id)
}

fn sound_label(asset_registry: &AssetRegistry, sound_id: SoundId) -> String {
    asset_registry
        .relative_path(sound_id)
        .map(|path| path.to_string_lossy().into_owned())
        .unwrap_or_else(|| format!("SoundId({})", sound_id.0))
}

fn stop_behavior_label(behavior: &AudioStopBehavior) -> String {
    match behavior {
        AudioStopBehavior::Immediate => "Immediate".to_string(),
        AudioStopBehavior::FadeOut { .. } => "Fade Out".to_string(),
    }
}

pub(crate) fn format_volume_label(volume: f32) -> String {
    format!("{volume:.VOLUME_LABEL_DECIMALS$}x")
}
