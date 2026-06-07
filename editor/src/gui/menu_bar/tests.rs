use super::*;
use crate::prefab::BLANK_PREFAB_ID;
use engine_core::worlds::*;
use engine_core::prefab::PrefabId;

#[test]
fn hierarchy_panel_action_is_limited_to_room_and_prefab_modes() {
    assert!(!EditorAction::ViewHierarchyPanel.is_available_in(EditorMode::Game));
    assert!(!EditorAction::ViewHierarchyPanel.is_available_in(EditorMode::World(WorldId(0))));
    assert!(EditorAction::ViewHierarchyPanel.is_available_in(EditorMode::Room(RoomId(2))));
    assert!(EditorAction::ViewHierarchyPanel.is_available_in(EditorMode::Prefab(PrefabId(7))));
    assert!(!EditorAction::ViewHierarchyPanel.is_available_in(EditorMode::Menu));
}

#[test]
fn prefab_palette_action_is_limited_to_room_mode() {
    assert!(EditorAction::ViewPrefabPalettePanel.is_available_in(EditorMode::Room(RoomId(2))));
    assert!(!EditorAction::ViewPrefabPalettePanel.is_available_in(EditorMode::Game));
    assert!(!EditorAction::ViewPrefabPalettePanel.is_available_in(EditorMode::Prefab(PrefabId(7))));
}

#[test]
fn prefab_browser_action_is_limited_to_prefab_modes() {
    assert!(
        EditorAction::ViewPrefabBrowserPanel.is_available_in(EditorMode::Prefab(BLANK_PREFAB_ID,))
    );
    assert!(EditorAction::ViewPrefabBrowserPanel.is_available_in(EditorMode::Prefab(PrefabId(7))));
    assert!(!EditorAction::ViewPrefabBrowserPanel.is_available_in(EditorMode::Room(RoomId(2))));
    assert_eq!(EditorAction::ViewPrefabBrowserPanel.shortcut(), Some("P"));
}

#[test]
fn blank_prefab_mode_has_back_action() {
    assert_eq!(back_action_for_mode(EditorMode::Prefab(BLANK_PREFAB_ID)), Some(EditorAction::NavigateBack));
}

#[cfg(debug_assertions)]
#[test]
fn file_menu_hides_change_save_root_in_debug_builds() {
    let actions = file_actions_for_mode(EditorMode::Game);

    assert!(!actions.contains(&EditorAction::ChangeSaveRoot));
}

#[test]
fn blank_prefab_mode_hides_save_and_rename_actions() {
    assert!(!EditorAction::Save.is_available_in(EditorMode::Prefab(BLANK_PREFAB_ID)));
    let file_actions = file_actions_for_mode(EditorMode::Prefab(BLANK_PREFAB_ID));

    assert!(!file_actions.contains(&EditorAction::Save));
    assert!(!file_actions.contains(&EditorAction::SaveAs));
    assert!(!EditorAction::Rename.is_available_in(EditorMode::Prefab(BLANK_PREFAB_ID)));
}

#[test]
fn prefab_mode_shows_hierarchy_in_view_menu() {
    let actions = view_actions_for_mode(EditorMode::Prefab(PrefabId(7)));

    assert!(actions.contains(&EditorAction::ViewHierarchyPanel));
}

#[test]
fn prefab_mode_shows_editor_settings_in_options_menu() {
    let actions = options_actions_for_mode(EditorMode::Prefab(PrefabId(7)));

    assert!(actions.contains(&EditorAction::EditorSettings));
}

#[test]
fn prefab_mode_shows_return_game_editor_in_editors_menu() {
    let actions = editors_actions_for_mode(EditorMode::Prefab(PrefabId(7)));

    assert_eq!(actions, vec![EditorAction::ReturnToGameEditor]);
}

#[cfg(not(debug_assertions))]
#[test]
fn file_menu_shows_change_save_root_in_release_builds() {
    let actions = file_actions_for_mode(EditorMode::Game);

    assert!(actions.contains(&EditorAction::ChangeSaveRoot));
}

#[test]
fn back_action_is_available_for_room_world_menu_and_prefab_modes() {
    assert_eq!(back_action_for_mode(EditorMode::Room(RoomId(1))), Some(EditorAction::NavigateBack));
    assert_eq!(back_action_for_mode(EditorMode::World(WorldId(1))), Some(EditorAction::NavigateBack));
    assert_eq!(back_action_for_mode(EditorMode::Menu), Some(EditorAction::NavigateBack));
    assert_eq!(back_action_for_mode(EditorMode::Prefab(PrefabId(7))), Some(EditorAction::NavigateBack));
}

#[test]
fn game_mode_has_no_back_action() {
    assert_eq!(back_action_for_mode(EditorMode::Game), None);
}

#[test]
fn inspector_toggle_action_is_available_in_inspector_modes_only() {
    assert!(EditorAction::ViewInspectorPanel.is_available_in(EditorMode::Game));
    assert!(EditorAction::ViewInspectorPanel.is_available_in(EditorMode::World(WorldId(0))));
    assert!(EditorAction::ViewInspectorPanel.is_available_in(EditorMode::Room(RoomId(1))));
    assert!(EditorAction::ViewInspectorPanel.is_available_in(EditorMode::Prefab(PrefabId(7))));
    assert!(!EditorAction::ViewInspectorPanel.is_available_in(EditorMode::Menu));
}

#[test]
fn room_mode_view_menu_contains_inspector_toggle() {
    let actions = view_actions_for_mode(EditorMode::Room(RoomId(1)));

    assert!(actions.contains(&EditorAction::ViewInspectorPanel));
}

#[test]
fn inspector_toggle_shortcut_matches_platform_convention() {
    #[cfg(target_os = "macos")]
    assert_eq!(EditorAction::ViewInspectorPanel.shortcut(), Some("⌘ I"));

    #[cfg(any(target_os = "windows", target_os = "linux"))]
    assert_eq!(EditorAction::ViewInspectorPanel.shortcut(), Some("^ I"));
}
