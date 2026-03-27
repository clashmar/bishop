use super::*;
use super::groups::{assignment_options, rename_target_group, AssignOption};
use super::preview::{
    active_preview_is_cleared_for_test, set_active_preview_for_test, ActivePreview,
};
use crate::storage::sound_preset_storage::set_current_sound_preset_library;
use engine_core::audio::audio_source::SoundPresetLink;

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

    assert_eq!(module.height(), TOP_PADDING + ROW_HEIGHT + SPACING);
}

#[test]
fn height_adds_only_rename_row_when_no_groups_and_rename_is_active() {
    let mut module = AudioSourceModule::default();
    module.pending_rename_target = Some(SoundGroupId::Custom("Group 1".to_string()));

    assert_eq!(module.height(), TOP_PADDING + ROW_HEIGHT * 2.0 + SPACING * 2.0);
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
        + ROW_HEIGHT;

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

    assert_eq!(with_preset_actions.height() - without_preset_actions.height(), ROW_HEIGHT + SPACING);
}
