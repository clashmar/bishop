use super::*;
use engine_core::audio::audio_source::SoundPresetLink;
use engine_core::audio::command_queue::{push_audio_command, AudioCommand};
use std::collections::BTreeSet;
use std::fmt::{Display, Formatter, Result as FmtResult};

#[derive(Clone, PartialEq)]
pub(super) enum AssignOption {
    AddEmpty,
    RenameCurrent,
    DuplicateCurrent,
    Preset(String),
}

impl AssignOption {
    pub(super) fn label(&self) -> String {
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
pub(super) enum PresetAction {
    Save(String),
    SyncFrom(String),
    Delete(String),
    Detach,
}

impl PresetAction {
    pub(super) fn label(&self) -> String {
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

pub(super) fn draw_group_dropdowns(
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
        if let Some(message) =
            handle_assign_option(source, choice, module, library, pending_sync_all)
        {
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

pub(super) fn draw_rename_field(
    ctx: &mut WgpuContext,
    blocked: bool,
    rect: Rect,
    module: &mut AudioSourceModule,
    source: &mut AudioSource,
    pending_link_rename: &mut Option<(String, String)>,
) -> Option<String> {
    let (entered, focused) =
        TextInput::new(module.rename_field_id, rect, &module.rename_initial_value)
            .focused(true)
            .blocked(blocked)
            .show(ctx);

    if ctx.is_key_pressed(KeyCode::Enter) {
        let result =
            rename_target_group(source, module.pending_rename_target.clone(), entered.trim());
        module.pending_rename_target = None;
        text_input_reset(module.rename_field_id);
        match result {
            Ok(link_rename) => {
                *pending_link_rename = link_rename;
                None
            }
            Err(message) => Some(message),
        }
    } else {
        if !focused {
            module.pending_rename_target = None;
            text_input_reset(module.rename_field_id);
        }

        None
    }
}

pub(super) fn assignment_options(
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
        .filter_map(|group| {
            group
                .preset_link
                .as_ref()
                .map(|link| link.preset_name.clone())
        })
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
                source
                    .groups
                    .insert(new_group_id.clone(), AudioGroup::default());
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

pub(super) fn preset_actions_for_group(
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

pub(super) fn handle_preset_action(
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

pub(super) fn preset_status_text(
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

pub(super) fn rename_target_group(
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

pub(super) fn apply_source_edit(source: &mut AudioSource, edit: impl FnOnce(&mut AudioSource)) {
    let before = source.all_sound_ids();
    edit(source);
    sync_sound_refs(&before, &source.all_sound_ids());
}

pub(super) fn sync_linked_groups_from_preset(
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

pub(super) fn rename_preset_links_in_ecs(
    ecs: &mut Ecs,
    old_preset_name: &str,
    new_preset_name: &str,
) {
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

        library.presets.insert(new_preset_name.to_string(), preset);
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

pub(super) fn ensure_selected_group(source: &mut AudioSource) {
    if source
        .current
        .as_ref()
        .is_some_and(|group_id| source.groups.contains_key(group_id))
    {
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
    source.groups.iter().find_map(|(group_id, group)| {
        group
            .preset_link
            .as_ref()
            .is_some_and(|link| link.preset_name == preset_name)
            .then(|| group_id.clone())
    })
}
