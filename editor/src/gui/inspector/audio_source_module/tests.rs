use super::groups::{
    assignment_options, handle_assign_option, handle_preset_action, preset_actions_for_group,
    rename_target_group, AssignOption, PresetAction,
};
use super::preview::{
    active_preview_is_cleared_for_test, set_active_preview_for_test, ActivePreview,
};
use super::*;
use crate::storage::sound_preset_storage::set_current_sound_preset_library;
use engine_core::audio::audio_source::SoundPresetLink;

#[test]
fn rename_target_group_renames_requested_group_even_if_selection_changes() {
    let mut source = AudioSource::default();
    let talk = SoundGroupId::Custom("Talk".to_string());
    let footsteps = SoundGroupId::Custom("Footsteps".to_string());
    source.groups.insert(talk.clone(), AudioGroup::default());
    source
        .groups
        .insert(footsteps.clone(), AudioGroup::default());
    source.current = Some(footsteps.clone());

    rename_target_group(&mut source, Some(talk.clone()), "Dialogue").unwrap();

    assert!(source
        .groups
        .contains_key(&SoundGroupId::Custom("Dialogue".to_string())));
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
        presets: std::collections::HashMap::from([("Jump".to_string(), AudioGroup::default())]),
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

    assert_eq!(link_rename, Some(("Jump".to_string(), "Leap".to_string())));
    assert!(current_sound_preset_library().presets.contains_key("Leap"));
    assert!(!current_sound_preset_library().presets.contains_key("Jump"));
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
fn assignment_options_keep_matching_preset_visible_for_detached_group() {
    let mut source = AudioSource::default();
    source.groups.insert(
        SoundGroupId::Custom("Jump".to_string()),
        AudioGroup::default(),
    );

    let library = crate::storage::sound_preset_storage::SoundPresetLibrary {
        presets: std::collections::HashMap::from([("Jump".to_string(), AudioGroup::default())]),
    };

    let options = assignment_options(&source, &library);

    assert!(options.contains(&AssignOption::Preset("Jump".to_string())));
}

#[test]
fn preset_actions_offer_reattach_for_detached_group_matching_preset() {
    let group_id = SoundGroupId::Custom("Jump".to_string());
    let group = AudioGroup::default();
    let library = crate::storage::sound_preset_storage::SoundPresetLibrary {
        presets: std::collections::HashMap::from([("Jump".to_string(), AudioGroup::default())]),
    };

    let labels = preset_actions_for_group(&group_id, &group, &library)
        .into_iter()
        .map(|action| action.label())
        .collect::<Vec<_>>();

    assert!(labels.contains(&"Reattach to Preset: Jump".to_string()));
}

#[test]
fn assigning_matching_preset_warns_when_component_already_has_group_with_same_name() {
    let mut source = AudioSource::default();
    let jump = SoundGroupId::Custom("Jump".to_string());
    let land = SoundGroupId::Custom("Land".to_string());
    source.groups.insert(jump.clone(), AudioGroup::default());
    source.groups.insert(land.clone(), AudioGroup::default());
    source.current = Some(land.clone());

    let library = crate::storage::sound_preset_storage::SoundPresetLibrary {
        presets: std::collections::HashMap::from([("Jump".to_string(), AudioGroup::default())]),
    };
    let mut module = AudioSourceModule::default();
    let mut pending_sync_all = None;

    let warning = handle_assign_option(
        &mut source,
        AssignOption::Preset("Jump".to_string()),
        &mut module,
        &library,
        &mut pending_sync_all,
    );

    assert_eq!(
        warning,
        Some(
            "This component already has a sound group named 'Jump'. Select that group and use Preset Actions to reattach it.".to_string()
        )
    );
    assert_eq!(source.current, Some(land));
    assert_eq!(source.groups.len(), 2);
    assert!(source.groups.contains_key(&jump));
    assert!(!source
        .groups
        .contains_key(&SoundGroupId::Custom("Jump 2".to_string())));
    assert!(pending_sync_all.is_none());
}

#[test]
fn reattach_action_applies_preset_and_restores_link() {
    set_current_sound_preset_library(crate::storage::sound_preset_storage::SoundPresetLibrary {
        presets: std::collections::HashMap::from([(
            "Jump".to_string(),
            AudioGroup {
                sounds: vec!["sfx/jump".to_string()],
                volume: 0.35,
                pitch_variation: 0.2,
                volume_variation: 0.1,
                looping: true,
                preset_link: None,
            },
        )]),
    });

    let mut source = AudioSource::default();
    let jump = SoundGroupId::Custom("Jump".to_string());
    source.groups.insert(
        jump.clone(),
        AudioGroup {
            sounds: vec!["sfx/local_jump".to_string()],
            volume: 0.9,
            pitch_variation: 0.0,
            volume_variation: 0.0,
            looping: false,
            preset_link: None,
        },
    );
    source.current = Some(jump.clone());

    let mut pending_sync_all = None;
    let warning = handle_preset_action(
        &mut source,
        PresetAction::Reattach("Jump".to_string()),
        &mut pending_sync_all,
    );

    assert_eq!(warning, None);
    assert!(pending_sync_all.is_none());

    let group = source.groups.get(&jump).unwrap();
    assert_eq!(group.sounds, vec!["sfx/jump".to_string()]);
    assert_eq!(group.volume, 0.35);
    assert_eq!(group.pitch_variation, 0.2);
    assert_eq!(group.volume_variation, 0.1);
    assert!(group.looping);
    assert_eq!(
        group
            .preset_link
            .as_ref()
            .map(|link| link.preset_name.as_str()),
        Some("Jump")
    );
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
    set_active_preview_for_test(Some(ActivePreview {
        entity: Entity(3),
        group_id: SoundGroupId::Custom("Jump".to_string()),
        request: PreviewRequest::new(0, "sfx/jump".to_string()),
        remaining_seconds: 0.25,
    }));

    tick_active_audio_preview(0.3);

    assert!(active_preview_is_cleared_for_test());
}

#[test]
fn height_matches_single_visible_row_when_source_has_no_groups() {
    let module = AudioSourceModule::default();

    assert_eq!(module.height(), TOP_PADDING + ROW_HEIGHT + SPACING + 5.0);
}

#[test]
fn height_adds_only_rename_row_when_no_groups_and_rename_is_active() {
    let module = AudioSourceModule {
        pending_rename_target: Some(SoundGroupId::Custom("Group 1".to_string())),
        ..Default::default()
    };

    assert_eq!(
        module.height(),
        TOP_PADDING + ROW_HEIGHT * 2.0 + SPACING * 2.0 + 5.0
    );
}

#[test]
fn height_matches_minimal_group_editor_without_sound_rows() {
    let module = AudioSourceModule {
        has_groups: true,
        sounds_len: 0,
        ..Default::default()
    };

    let expected = TOP_PADDING
        + ROW_HEIGHT
        + SPACING
        + SECTION_GAP
        + ROW_HEIGHT
        + EDIT_SECTION_SPACING
        + ROW_HEIGHT
        + EDIT_SECTION_SPACING
        + ROW_HEIGHT
        + EDIT_SECTION_SPACING
        + ROW_HEIGHT
        + EDIT_SECTION_SPACING
        + ROW_HEIGHT
        + 5.0;

    assert_eq!(module.height(), expected);
}

#[test]
fn height_includes_preset_actions_row_only_when_cached_as_visible() {
    let without_preset_actions = AudioSourceModule {
        has_groups: true,
        sounds_len: 0,
        ..Default::default()
    };
    let with_preset_actions = AudioSourceModule {
        has_groups: true,
        has_preset_actions: true,
        sounds_len: 0,
        ..Default::default()
    };

    assert_eq!(
        with_preset_actions.height() - without_preset_actions.height(),
        ROW_HEIGHT + SPACING
    );
}
