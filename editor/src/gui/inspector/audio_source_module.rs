use crate::storage::sound_preset_storage::*;
use engine_core::audio::command_queue::{push_audio_command, AudioCommand};
use engine_core::audio::audio_source::SoundPresetLink;
use engine_core::prelude::*;
use bishop::prelude::*;
use std::cell::RefCell;
use std::collections::BTreeSet;
use std::fmt::{Display, Formatter, Result as FmtResult};

const TOP_PADDING: f32 = 10.0;
const SPACING: f32 = 5.0;
const SECTION_GAP: f32 = 12.0;
const EDIT_SECTION_SPACING: f32 = 9.0;
const ROW_HEIGHT: f32 = DEFAULT_FIELD_HEIGHT;
const LABEL_W: f32 = 80.0;
const VOLUME_LABEL_DECIMALS: usize = 2;
const PREVIEW_HANDLE: u64 = 0x4544_4954_4F52_5052;
const PREVIEW_TIMEOUT_SECONDS: f32 = 5.0;

#[derive(Clone, Debug, PartialEq, Eq)]
struct PreviewRequest {
    row_index: usize,
    sound_id: String,
}

impl PreviewRequest {
    fn new(row_index: usize, sound_id: String) -> Self {
        Self { row_index, sound_id }
    }
}

#[derive(Clone, Debug, PartialEq)]
struct ActivePreview {
    entity: Entity,
    group_id: SoundGroupId,
    request: PreviewRequest,
    remaining_seconds: f32,
}

thread_local! {
    static ACTIVE_AUDIO_PREVIEW: RefCell<Option<ActivePreview>> = const { RefCell::new(None) };
}

#[derive(Clone, PartialEq)]
enum AssignOption {
    AddEmpty,
    RenameCurrent,
    DuplicateCurrent,
    Preset(String),
}

impl AssignOption {
    fn label(&self) -> String {
        match self {
            Self::AddEmpty => "Add Empty Group".to_string(),
            Self::RenameCurrent => "Rename Group".to_string(),
            Self::DuplicateCurrent => "Duplicate Group".to_string(),
            Self::Preset(name) => format!("Use Preset: {name}"),
        }
    }
}

impl Display for AssignOption {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        f.write_str(&self.label())
    }
}

#[derive(Clone, PartialEq)]
enum PresetAction {
    Save(String),
    SyncFrom(String),
    Delete(String),
    Detach,
}

impl PresetAction {
    fn label(&self) -> String {
        match self {
            Self::Save(name) => format!("Save Preset: {name}"),
            Self::SyncFrom(name) => format!("Sync From Preset: {name}"),
            Self::Delete(name) => format!("Delete Preset: {name}"),
            Self::Detach => "Detach Preset".to_string(),
        }
    }
}

impl Display for PresetAction {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        f.write_str(&self.label())
    }
}

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

    fn height(&self) -> f32 {
        let mut rows = self.sounds_len + 8;
        if self.pending_rename_target.is_some() {
            rows += 1;
        }
        if !self.has_groups {
            rows = if self.pending_rename_target.is_some() { 3 } else { 2 };
        }
        TOP_PADDING
            + rows as f32 * (ROW_HEIGHT + SPACING)
            + SPACING
            + if self.has_groups { SECTION_GAP } else { 0.0 }
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
                self.sounds_len = 0;
                self.update_warning(ctx, warning_message);
                return;
            };

            let Some(group) = source.groups.get(&current_group_id) else {
                clear_active_audio_preview();
                self.sounds_len = 0;
                self.update_warning(ctx, Some("Current sound group is missing".to_string()));
                return;
            };

            let status_text = preset_status_text(group, &library);
            let preset_actions = preset_actions_for_group(&current_group_id, group, &library);
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
            if Button::new(add_rect, "Add Sound").blocked(blocked).show(ctx) {
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

pub fn clear_active_audio_preview() {
    ACTIVE_AUDIO_PREVIEW.with(|active| {
        if active.borrow_mut().take().is_some() {
            push_audio_command(AudioCommand::StopTrackedPreview(PREVIEW_HANDLE));
        }
    });
}

fn tick_active_audio_preview(dt: f32) {
    let expired = ACTIVE_AUDIO_PREVIEW.with(|active| {
        let mut active = active.borrow_mut();
        let Some(preview) = active.as_mut() else {
            return false;
        };

        preview.remaining_seconds -= dt.max(0.0);
        preview.remaining_seconds <= 0.0
    });

    if expired {
        clear_active_audio_preview();
    }
}

fn format_volume_label(volume: f32) -> String {
    format!("{volume:.VOLUME_LABEL_DECIMALS$}x")
}

fn sync_active_preview(
    entity: Entity,
    group_id: &SoundGroupId,
    sounds: &[String],
) {
    let should_clear = ACTIVE_AUDIO_PREVIEW.with(|active| {
        active.borrow().as_ref().is_some_and(|preview| {
            preview.entity == entity
                && (preview.group_id != *group_id
                    || !sounds.iter().enumerate().any(|(index, sound)| {
                        preview.request.row_index == index && preview.request.sound_id == *sound
                    }))
        })
    });

    if should_clear {
        clear_active_audio_preview();
    }
}

fn apply_preview_request(
    entity: Entity,
    group_id: &SoundGroupId,
    next_preview: Option<PreviewRequest>,
    group: &AudioGroup,
) {
    match next_preview {
        Some(request) => {
            push_audio_command(AudioCommand::PlayTrackedPreview {
                handle: PREVIEW_HANDLE,
                sounds: vec![request.sound_id.clone()],
                volume: group.volume,
                pitch_variation: group.pitch_variation,
                volume_variation: group.volume_variation,
                looping: false,
                timeout: PREVIEW_TIMEOUT_SECONDS,
            });
            ACTIVE_AUDIO_PREVIEW.with(|active| {
                *active.borrow_mut() = Some(ActivePreview {
                    entity,
                    group_id: group_id.clone(),
                    request,
                    remaining_seconds: PREVIEW_TIMEOUT_SECONDS,
                });
            });
        }
        None => clear_active_audio_preview(),
    }
}

fn draw_group_dropdowns(
    ctx: &mut WgpuContext,
    blocked: bool,
    rect: Rect,
    module: &mut AudioSourceModule,
    source: &mut AudioSource,
    library: &crate::storage::sound_preset_storage::SoundPresetLibrary,
    warning_message: &mut Option<String>,
    pending_sync_all: &mut Option<(String, AudioGroup)>,
) {
    let current_group_label = source
        .current
        .as_ref()
        .map_or_else(|| "No Group".to_string(), SoundGroupId::ui_label);
    let options = existing_group_ids(source);
    let assign_options = assignment_options(source, library);

    let select_width = ((rect.w - ROW_HEIGHT - SPACING * 2.0) * 0.5).max(0.0);
    let assign_width = (rect.w - select_width - ROW_HEIGHT - SPACING * 2.0).max(0.0);

    let select_rect = Rect::new(rect.x, rect.y, select_width, rect.h);
    let assign_rect = Rect::new(
        select_rect.x + select_rect.w + SPACING,
        rect.y,
        assign_width,
        rect.h,
    );
    let remove_rect = Rect::new(
        assign_rect.x + assign_rect.w + SPACING,
        rect.y,
        ROW_HEIGHT,
        ROW_HEIGHT,
    );

    if let Some(selected) = Dropdown::new(
        module.select_dropdown_id,
        select_rect,
        &current_group_label,
        &options,
        SoundGroupId::ui_label,
    )
    .list_width(rect.w)
    .truncate_trigger_text()
    .blocked(blocked)
    .show(ctx)
    {
        source.current = Some(selected);
    }

    if let Some(choice) = Dropdown::new(
        module.assign_dropdown_id,
        assign_rect,
        "Add / Assign",
        &assign_options,
        AssignOption::label,
    )
    .right_aligned()
    .blocked(blocked)
    .show(ctx)
    {
        if let Some(message) = handle_assign_option(
            source,
            choice,
            module,
            library,
            pending_sync_all,
        ) {
            *warning_message = Some(message);
        }
    }

    if Button::new(remove_rect, "x")
        .blocked(blocked || source.current.is_none())
        .show(ctx)
    {
        apply_source_edit(source, |source| {
            if let Some(current) = source.current.take() {
                source.groups.remove(&current);
                source.current = first_group_id(source);
            }
        });
    }
}

fn draw_rename_field(
    ctx: &mut WgpuContext,
    blocked: bool,
    rect: Rect,
    module: &mut AudioSourceModule,
    source: &mut AudioSource,
    pending_link_rename: &mut Option<(String, String)>,
) -> Option<String> {
    let (entered, focused) = TextInput::new(module.rename_field_id, rect, &module.rename_initial_value)
        .focused(true)
        .blocked(blocked)
        .show(ctx);

    if ctx.is_key_pressed(KeyCode::Enter) {
        let result = rename_target_group(source, module.pending_rename_target.clone(), entered.trim());
        module.pending_rename_target = None;
        text_input_reset(module.rename_field_id);
        match result {
            Ok(link_rename) => {
                *pending_link_rename = link_rename;
                return None;
            }
            Err(message) => return Some(message),
        }
    }

    if !focused {
        module.pending_rename_target = None;
        text_input_reset(module.rename_field_id);
    }

    None
}

fn assignment_options(
    source: &AudioSource,
    library: &crate::storage::sound_preset_storage::SoundPresetLibrary,
) -> Vec<AssignOption> {
    let mut options = vec![AssignOption::AddEmpty];

    if source.current.is_some() {
        options.push(AssignOption::RenameCurrent);
        options.push(AssignOption::DuplicateCurrent);
    }

    let used_presets = source
        .groups
        .values()
        .filter_map(|group| group.preset_link.as_ref().map(|link| link.preset_name.clone()))
        .collect::<BTreeSet<_>>();

    let mut preset_names = library
        .presets
        .keys()
        .filter(|name| !used_presets.contains(*name))
        .cloned()
        .collect::<Vec<_>>();
    preset_names.sort();
    options.extend(preset_names.into_iter().map(AssignOption::Preset));
    options
}

fn handle_assign_option(
    source: &mut AudioSource,
    choice: AssignOption,
    module: &mut AudioSourceModule,
    library: &crate::storage::sound_preset_storage::SoundPresetLibrary,
    pending_sync_all: &mut Option<(String, AudioGroup)>,
) -> Option<String> {
    match choice {
        AssignOption::AddEmpty => {
            let new_name = next_group_name(source);
            let new_group_id = SoundGroupId::Custom(new_name.clone());
            apply_source_edit(source, |source| {
                source.groups.insert(new_group_id.clone(), AudioGroup::default());
                source.current = Some(new_group_id.clone());
            });
            module.pending_rename_target = Some(new_group_id);
            module.rename_initial_value = new_name;
            None
        }
        AssignOption::RenameCurrent => {
            let Some(group_id) = source.current.clone() else {
                return Some("Only custom groups can be renamed".to_string());
            };
            let Some(name) = group_id_name(&group_id).map(str::to_string) else {
                return Some("Only custom groups can be renamed".to_string());
            };
            module.pending_rename_target = Some(group_id);
            module.rename_initial_value = name;
            None
        }
        AssignOption::DuplicateCurrent => {
            let Some(current_group_id) = source.current.clone() else {
                return Some("Select a sound group before duplicating".to_string());
            };
            let Some(group) = source.groups.get(&current_group_id).cloned() else {
                return Some("Current sound group is missing".to_string());
            };
            let base_name = group_id_name(&current_group_id)
                .map(|name| format!("{name} Copy"))
                .unwrap_or_else(|| "Group Copy".to_string());
            let new_name = unique_group_name(source, &base_name);
            let new_group_id = SoundGroupId::Custom(new_name.clone());

            apply_source_edit(source, |source| {
                let mut detached_group = group.clone();
                detached_group.preset_link = None;
                source.groups.insert(new_group_id.clone(), detached_group);
                source.current = Some(new_group_id.clone());
            });

            module.pending_rename_target = Some(new_group_id);
            module.rename_initial_value = new_name;
            None
        }
        AssignOption::Preset(preset_name) => {
            if let Some(existing_group_id) = find_group_linked_to_preset(source, &preset_name) {
                source.current = Some(existing_group_id);
                return None;
            }

            let Some(preset) = library.presets.get(&preset_name).cloned() else {
                return Some(format!("Missing preset: {preset_name}"));
            };
            let group_name = unique_group_name(source, &preset_name);

            apply_source_edit(source, |source| {
                let new_id = SoundGroupId::Custom(group_name.clone());
                let mut group = AudioGroup::default();
                group.apply_preset(&preset_name, &preset);
                source.groups.insert(new_id.clone(), group);
                source.current = Some(new_id);
            });

            *pending_sync_all = Some((preset_name, preset));
            None
        }
    }
}

fn preset_actions_for_group(
    current_group_id: &SoundGroupId,
    group: &AudioGroup,
    library: &crate::storage::sound_preset_storage::SoundPresetLibrary,
) -> Vec<PresetAction> {
    let mut actions = Vec::new();

    if let Some(link) = &group.preset_link {
        if library.presets.contains_key(&link.preset_name) {
            actions.push(PresetAction::Save(link.preset_name.clone()));
            actions.push(PresetAction::SyncFrom(link.preset_name.clone()));
            actions.push(PresetAction::Delete(link.preset_name.clone()));
        }
        actions.push(PresetAction::Detach);
        return actions;
    }

    if let Some(group_name) = group_id_name(current_group_id) {
        if !library.presets.contains_key(group_name) {
            actions.push(PresetAction::Save(group_name.to_string()));
        }
    }

    actions
}

fn handle_preset_action(
    source: &mut AudioSource,
    action: PresetAction,
    pending_sync_all: &mut Option<(String, AudioGroup)>,
) -> Option<String> {
    let Some(current_group_id) = source.current.clone() else {
        return Some("Select a sound group first".to_string());
    };
    let Some(current_group) = source.groups.get(&current_group_id).cloned() else {
        return Some("Current sound group is missing".to_string());
    };

    match action {
        PresetAction::Save(preset_name) => {
            let mut preset = current_group.clone();
            preset.preset_link = None;

            with_sound_preset_library_mut(|library| {
                library.presets.insert(preset_name.clone(), preset.clone());
            });
            apply_source_edit(source, |source| {
                if let Some(group) = source.groups.get_mut(&current_group_id) {
                    group.preset_link = Some(SoundPresetLink {
                        preset_name: preset_name.clone(),
                    });
                }
            });
            *pending_sync_all = Some((preset_name, preset));
            None
        }
        PresetAction::SyncFrom(preset_name) => {
            let Some(preset) = current_sound_preset_library()
                .presets
                .get(&preset_name)
                .cloned()
            else {
                return Some(format!("Missing preset: {preset_name}"));
            };

            apply_source_edit(source, |source| {
                if let Some(group) = source.groups.get_mut(&current_group_id) {
                    group.apply_preset(&preset_name, &preset);
                }
            });
            None
        }
        PresetAction::Delete(preset_name) => {
            if !delete_sound_preset(&preset_name) {
                return Some(format!("Missing preset: {preset_name}"));
            }

            apply_source_edit(source, |source| {
                if let Some(group) = source.groups.get_mut(&current_group_id) {
                    group.preset_link = None;
                }
            });

            None
        }
        PresetAction::Detach => {
            apply_source_edit(source, |source| {
                if let Some(group) = source.groups.get_mut(&current_group_id) {
                    group.preset_link = None;
                }
            });
            None
        }
    }
}

fn preset_status_text(
    group: &AudioGroup,
    library: &crate::storage::sound_preset_storage::SoundPresetLibrary,
) -> String {
    match &group.preset_link {
        Some(link) if library.presets.contains_key(&link.preset_name) => {
            format!("Linked: {}", link.preset_name)
        }
        Some(link) => format!("Missing Preset: {}", link.preset_name),
        None => "Detached".to_string(),
    }
}

fn rename_target_group(
    source: &mut AudioSource,
    target_group_id: Option<SoundGroupId>,
    new_name: &str,
) -> Result<Option<(String, String)>, String> {
    let trimmed = new_name.trim();
    if trimmed.is_empty() {
        return Err("Sound group name cannot be empty".to_string());
    }

    let Some(target_group_id) = target_group_id else {
        return Err("No sound group is pending rename".to_string());
    };
    let Some(current_name) = group_id_name(&target_group_id) else {
        return Err("Only custom groups can be renamed".to_string());
    };

    if current_name == trimmed {
        return Ok(None);
    }

    let new_group_id = SoundGroupId::Custom(trimmed.to_string());
    if source.groups.contains_key(&new_group_id) {
        return Err(format!("Sound group '{trimmed}' already exists"));
    }

    let Some(group) = source.groups.remove(&target_group_id) else {
        return Err("Pending rename group is missing".to_string());
    };

    let mut group = group;
    let link_rename = rename_linked_preset_for_group(&mut group, trimmed)?;

    source.groups.insert(new_group_id.clone(), group);
    if source.current.as_ref() == Some(&target_group_id) {
        source.current = Some(new_group_id);
    }
    Ok(link_rename)
}

fn apply_source_edit(
    source: &mut AudioSource,
    edit: impl FnOnce(&mut AudioSource),
) {
    let before = source.all_sound_ids();
    edit(source);
    sync_sound_refs(&before, &source.all_sound_ids());
}

fn sync_linked_groups_from_preset(
    ecs: &mut Ecs,
    preset_name: &str,
    preset: &AudioGroup,
) {
    let store = ecs.get_store_mut::<AudioSource>();

    for source in store.data.values_mut() {
        let before = source.all_sound_ids();
        let mut changed = false;

        for group in source.groups.values_mut() {
            if group
                .preset_link
                .as_ref()
                .is_some_and(|link| link.preset_name == preset_name)
            {
                group.apply_preset(preset_name, preset);
                changed = true;
            }
        }

        if changed {
            sync_sound_refs(&before, &source.all_sound_ids());
        }
    }
}

fn rename_preset_links_in_ecs(ecs: &mut Ecs, old_preset_name: &str, new_preset_name: &str) {
    if old_preset_name == new_preset_name {
        return;
    }

    for source in ecs.get_store_mut::<AudioSource>().data.values_mut() {
        for group in source.groups.values_mut() {
            if let Some(link) = &mut group.preset_link {
                if link.preset_name == old_preset_name {
                    link.preset_name = new_preset_name.to_string();
                }
            }
        }
    }
}

fn rename_linked_preset_for_group(
    group: &mut AudioGroup,
    new_preset_name: &str,
) -> Result<Option<(String, String)>, String> {
    let Some(link) = &mut group.preset_link else {
        return Ok(None);
    };

    let old_preset_name = link.preset_name.clone();
    if old_preset_name == new_preset_name {
        return Ok(None);
    }

    with_sound_preset_library_mut(|library| {
        if library.presets.contains_key(new_preset_name) {
            return Err(format!("Preset '{new_preset_name}' already exists"));
        }

        let Some(preset) = library.presets.remove(&old_preset_name) else {
            link.preset_name = new_preset_name.to_string();
            return Ok(Some((old_preset_name.clone(), new_preset_name.to_string())));
        };

        library
            .presets
            .insert(new_preset_name.to_string(), preset);
        link.preset_name = new_preset_name.to_string();
        Ok(Some((old_preset_name.clone(), new_preset_name.to_string())))
    })
}

fn sync_sound_refs(before: &[String], after: &[String]) {
    let before_set = before.iter().cloned().collect::<BTreeSet<_>>();
    let after_set = after.iter().cloned().collect::<BTreeSet<_>>();

    let added = after_set
        .difference(&before_set)
        .cloned()
        .collect::<Vec<_>>();
    if !added.is_empty() {
        push_audio_command(AudioCommand::IncrementRefs(added));
    }

    let removed = before_set
        .difference(&after_set)
        .cloned()
        .collect::<Vec<_>>();
    if !removed.is_empty() {
        push_audio_command(AudioCommand::DecrementRefs(removed));
    }
}

fn ensure_selected_group(source: &mut AudioSource) {
    if source.current.as_ref().is_some_and(|group_id| source.groups.contains_key(group_id)) {
        return;
    }

    source.current = first_group_id(source);
}

fn existing_group_ids(source: &AudioSource) -> Vec<SoundGroupId> {
    let mut groups = source.groups.keys().cloned().collect::<Vec<_>>();
    groups.sort();
    groups
}

fn first_group_id(source: &AudioSource) -> Option<SoundGroupId> {
    source.groups.keys().cloned().min()
}

fn group_id_name(group_id: &SoundGroupId) -> Option<&str> {
    match group_id {
        SoundGroupId::Custom(name) => Some(name.as_str()),
        SoundGroupId::New => None,
    }
}

fn next_group_name(source: &AudioSource) -> String {
    let mut index = 1;
    loop {
        let candidate = format!("Group {index}");
        if !source
            .groups
            .contains_key(&SoundGroupId::Custom(candidate.clone()))
        {
            return candidate;
        }
        index += 1;
    }
}

fn unique_group_name(source: &AudioSource, base_name: &str) -> String {
    if !source
        .groups
        .contains_key(&SoundGroupId::Custom(base_name.to_string()))
    {
        return base_name.to_string();
    }

    let mut index = 2;
    loop {
        let candidate = format!("{base_name} {index}");
        if !source
            .groups
            .contains_key(&SoundGroupId::Custom(candidate.clone()))
        {
            return candidate;
        }
        index += 1;
    }
}

fn find_group_linked_to_preset(source: &AudioSource, preset_name: &str) -> Option<SoundGroupId> {
    source
        .groups
        .iter()
        .find_map(|(group_id, group)| {
            group.preset_link
                .as_ref()
                .is_some_and(|link| link.preset_name == preset_name)
                .then(|| group_id.clone())
        })
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
mod tests {
    use super::*;
    use crate::storage::sound_preset_storage::set_current_sound_preset_library;

    #[test]
    fn rename_target_group_renames_requested_group_even_if_selection_changes() {
        let mut source = AudioSource::default();
        let talk = SoundGroupId::Custom("Talk".to_string());
        let footsteps = SoundGroupId::Custom("Footsteps".to_string());
        source.groups.insert(talk.clone(), AudioGroup::default());
        source.groups.insert(footsteps.clone(), AudioGroup::default());
        source.current = Some(footsteps.clone());

        rename_target_group(&mut source, Some(talk.clone()), "Dialogue").unwrap();

        assert!(source.groups.contains_key(&SoundGroupId::Custom("Dialogue".to_string())));
        assert!(!source.groups.contains_key(&talk));
        assert_eq!(source.current, Some(footsteps));
    }

    #[test]
    fn rename_target_group_errors_when_target_group_was_removed() {
        let mut source = AudioSource::default();
        let talk = SoundGroupId::Custom("Talk".to_string());

        let error = rename_target_group(&mut source, Some(talk), "Dialogue").unwrap_err();

        assert_eq!(error, "Pending rename group is missing".to_string());
    }

    #[test]
    fn rename_target_group_renames_linked_preset_and_returns_link_update() {
        set_current_sound_preset_library(crate::storage::sound_preset_storage::SoundPresetLibrary {
            presets: std::collections::HashMap::from([(
                "Jump".to_string(),
                AudioGroup::default(),
            )]),
        });

        let mut source = AudioSource::default();
        let jump = SoundGroupId::Custom("Jump".to_string());
        source.groups.insert(
            jump.clone(),
            AudioGroup {
                preset_link: Some(SoundPresetLink {
                    preset_name: "Jump".to_string(),
                }),
                ..Default::default()
            },
        );
        source.current = Some(jump.clone());

        let link_rename = rename_target_group(&mut source, Some(jump), "Leap").unwrap();

        assert_eq!(
            link_rename,
            Some(("Jump".to_string(), "Leap".to_string()))
        );
        assert!(
            current_sound_preset_library()
                .presets
                .contains_key("Leap")
        );
        assert!(
            !current_sound_preset_library()
                .presets
                .contains_key("Jump")
        );
        assert_eq!(
            source
                .groups
                .get(&SoundGroupId::Custom("Leap".to_string()))
                .and_then(|group| group.preset_link.as_ref())
                .map(|link| link.preset_name.clone()),
            Some("Leap".to_string())
        );
    }

    #[test]
    fn assignment_options_omits_presets_already_linked_on_entity() {
        let mut source = AudioSource::default();
        source.groups.insert(
            SoundGroupId::Custom("Jump".to_string()),
            AudioGroup {
                preset_link: Some(SoundPresetLink {
                    preset_name: "Jump".to_string(),
                }),
                ..Default::default()
            },
        );

        let library = crate::storage::sound_preset_storage::SoundPresetLibrary {
            presets: std::collections::HashMap::from([
                ("Jump".to_string(), AudioGroup::default()),
                ("Land".to_string(), AudioGroup::default()),
            ]),
        };

        let options = assignment_options(&source, &library);

        assert!(!options.contains(&AssignOption::Preset("Jump".to_string())));
        assert!(options.contains(&AssignOption::Preset("Land".to_string())));
    }

    #[test]
    fn format_volume_label_uses_two_decimal_place_multiplier() {
        assert_eq!(format_volume_label(1.0), "1.00x");
        assert_eq!(format_volume_label(0.25), "0.25x");
    }

    #[test]
    fn preview_request_keeps_requested_sound_and_row() {
        let next = PreviewRequest::new(7, "sfx/land".to_string());

        assert_eq!(next, PreviewRequest::new(7, "sfx/land".to_string()));
    }

    #[test]
    fn tick_active_audio_preview_clears_expired_preview() {
        ACTIVE_AUDIO_PREVIEW.with(|active| {
            *active.borrow_mut() = Some(ActivePreview {
                entity: Entity(3),
                group_id: SoundGroupId::Custom("Jump".to_string()),
                request: PreviewRequest::new(0, "sfx/jump".to_string()),
                remaining_seconds: 0.25,
            });
        });

        tick_active_audio_preview(0.3);

        ACTIVE_AUDIO_PREVIEW.with(|active| {
            assert!(active.borrow().is_none());
        });
    }
}
