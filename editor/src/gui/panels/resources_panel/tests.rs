use crate::app::{Editor, EditorMode};
use crate::storage::editor_storage::create_new_game;
use crate::test_utils::{game_fs_test_lock, make_prefab_session_editor, TestGameFolder};
use engine_core::assets::{AssetKey, AssetRegistry};
use engine_core::constants::{extensions, paths};
use engine_core::ecs::ScriptId;
use engine_core::engine_global::set_game_name;
use engine_core::scripting::lua_constants::lua_dirs;
use engine_core::storage::path_utils::resources_folder_current;

use super::content_space_mouse_position;
use super::context_menu::{
    self, context_target_for_entry, open_resource, ActiveMenu, EntryKind, PendingResourceAction,
    ResourceMenuAction, ResourceOpenResult,
};
use super::icon_mapper::{IconMapper, IconType, FILE_ICON_MAP};
use super::navigation::Navigation;
use super::path_filter::{PathFilter, HIDDEN_DIRS, HIDDEN_EXTENSIONS, HIDDEN_FILENAMES};
use super::Entry;
use super::ResourcesPanel;
use bishop::prelude::*;
use std::collections::BTreeSet;
use std::path::PathBuf;

fn test_entry(name: &str, kind: EntryKind) -> Entry {
    Entry {
        name: name.to_string(),
        display_name: name.to_string(),
        kind,
        path: PathBuf::from(name),
        icon_type: if matches!(
            kind,
            EntryKind::Parent | EntryKind::Directory | EntryKind::SystemDirectory
        ) {
            IconType::Folder
        } else {
            IconType::File
        },
    }
}

#[test]
fn dir_visible_hides_hidden_dirs() {
    for &name in HIDDEN_DIRS {
        assert!(!PathFilter::dir_visible(name), "should hide dir: {name}");
    }
}

#[test]
fn file_visible_hides_language_manifest() {
    assert!(!PathFilter::file_visible(paths::LANGUAGE_MANIFEST));
}

#[test]
fn dir_visible_hides_engine_dir() {
    assert!(!PathFilter::dir_visible(lua_dirs::ENGINE));
}

#[test]
fn dir_visible_allows_unknown() {
    assert!(PathFilter::dir_visible("my_custom_dir"));
}

#[test]
fn file_visible_hides_hidden_filenames() {
    for &name in HIDDEN_FILENAMES {
        assert!(!PathFilter::file_visible(name), "should hide file: {name}");
    }
}

#[test]
fn file_visible_hides_hidden_extensions() {
    for &ext in HIDDEN_EXTENSIONS {
        let filename = format!("test_file.{ext}");
        assert!(
            !PathFilter::file_visible(&filename),
            "should hide .{ext} file"
        );
    }
}

#[test]
fn file_visible_allows_unknown_extension() {
    assert!(PathFilter::file_visible("readme.txt"));
}

#[test]
fn file_visible_hides_dotfiles() {
    assert!(!PathFilter::file_visible(".DS_Store"));
    assert!(!PathFilter::file_visible(".gitkeep"));
    assert!(!PathFilter::file_visible(".hidden"));
}

#[test]
fn dir_icon_returns_folder() {
    assert_eq!(IconMapper::dir_icon(), IconType::Folder);
}

#[test]
fn file_icon_maps_known_extensions() {
    for &(ext, expected) in FILE_ICON_MAP {
        let filename = format!("test_file.{ext}");
        assert_eq!(
            IconMapper::file_icon(&filename),
            expected,
            "file_icon(.{ext})"
        );
    }
}

#[test]
fn file_icon_unknown_extension_gets_file() {
    assert_eq!(IconMapper::file_icon("data.dat"), IconType::File);
}

#[test]
fn file_icon_no_extension_gets_file() {
    assert_eq!(IconMapper::file_icon("Makefile"), IconType::File);
}

#[test]
fn file_icon_maps_prefab_extension_to_prefab() {
    let filename = format!("test_file.{}", extensions::PREFAB);

    assert_eq!(IconMapper::file_icon(&filename), IconType::Prefab);
}

#[test]
fn file_icon_maps_ron_extension_to_file() {
    let filename = format!("test_file.{}", extensions::RON);

    assert_eq!(IconMapper::file_icon(&filename), IconType::File);
}

#[test]
fn navigation_starts_at_root() {
    let nav = Navigation::new();
    assert!(nav.is_at_root());
}

#[test]
fn navigation_push_goes_into_subdirectory() {
    let mut nav = Navigation::new();
    nav.push("assets");
    assert!(!nav.is_at_root());
}

#[test]
fn navigation_pop_goes_back_to_parent() {
    let mut nav = Navigation::new();
    nav.push("assets");
    let went_back = nav.pop();
    assert!(went_back);
    assert!(nav.is_at_root());
}

#[test]
fn navigation_pop_at_root_returns_false() {
    let mut nav = Navigation::new();
    let went_back = nav.pop();
    assert!(!went_back);
    assert!(nav.is_at_root());
}

#[test]
fn navigation_deep_path_push_pop() {
    let mut nav = Navigation::new();
    nav.push("assets");
    nav.push("sprites");
    nav.push("tiles");
    assert!(!nav.is_at_root());
    nav.pop();
    assert!(!nav.is_at_root());
    nav.pop();
    assert!(!nav.is_at_root());
    nav.pop();
    assert!(nav.is_at_root());
}

#[test]
fn navigation_depth_reflects_segment_count() {
    let mut nav = Navigation::new();
    assert_eq!(nav.depth(), 0);
    nav.push("assets");
    assert_eq!(nav.depth(), 1);
    nav.push("sprites");
    assert_eq!(nav.depth(), 2);
    nav.pop();
    assert_eq!(nav.depth(), 1);
}

#[test]
fn navigation_truncate_to_root() {
    let mut nav = Navigation::new();
    nav.push("assets");
    nav.push("sprites");
    nav.push("tiles");
    nav.truncate_to(0);
    assert!(nav.is_at_root());
    assert_eq!(nav.depth(), 0);
}

#[test]
fn navigation_truncate_to_mid_depth() {
    let mut nav = Navigation::new();
    nav.push("assets");
    nav.push("sprites");
    nav.push("tiles");
    nav.truncate_to(1);
    assert_eq!(nav.depth(), 1);
    assert_eq!(nav.segment(0), Some("assets"));
}

#[test]
fn navigation_truncate_to_current_depth_is_noop() {
    let mut nav = Navigation::new();
    nav.push("assets");
    nav.push("sprites");
    nav.truncate_to(2);
    assert_eq!(nav.depth(), 2);
}

#[test]
fn navigation_segment_returns_correct_value() {
    let mut nav = Navigation::new();
    nav.push("assets");
    nav.push("sprites");
    assert_eq!(nav.segment(0), Some("assets"));
    assert_eq!(nav.segment(1), Some("sprites"));
    assert_eq!(nav.segment(2), None);
}

fn setup_test_game(test_prefix: &str) -> (TestGameFolder, impl Drop) {
    let lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_game = TestGameFolder::new(test_prefix);
    set_game_name(test_game.name());
    let resources = resources_folder_current();
    std::fs::create_dir_all(resources.join("subdir")).unwrap();
    std::fs::create_dir_all(resources.join("subdir").join("nested")).unwrap();
    std::fs::write(resources.join("subdir").join("test.lua"), "").unwrap();
    (test_game, lock)
}

#[test]
fn scan_at_root_has_no_parent_entry() {
    let (_test_game, _lock) = setup_test_game("resources_panel_no_parent_at_root");
    let mut panel = ResourcesPanel::new();
    panel.scan_current_dir(&AssetRegistry::default());
    assert!(panel.navigation.is_at_root());
    assert!(panel.entries.first().is_none_or(|e| !e.is_parent()));
}

#[test]
fn scan_in_subdir_has_parent_as_first_entry() {
    let (_test_game, _lock) = setup_test_game("resources_panel_parent_first");
    let mut panel = ResourcesPanel::new();
    panel.navigation.push("subdir");
    panel.scan_current_dir(&AssetRegistry::default());

    let first = panel.entries.first().expect("should have a parent entry");
    assert!(first.is_parent());
    assert_eq!(first.display_name, "..");
    assert!(first.is_dir_like());
    assert!(!first.is_registered());
}

#[test]
fn clicking_parent_entry_navigates_to_root() {
    let (_test_game, _lock) = setup_test_game("resources_panel_click_parent");
    let mut panel = ResourcesPanel::new();
    panel.navigation.push("subdir");
    panel.scan_current_dir(&AssetRegistry::default());

    assert!(!panel.navigation.is_at_root());
    panel.navigation.pop();
    panel.scan_current_dir(&AssetRegistry::default());
    assert!(panel.navigation.is_at_root());
    assert!(panel.entries.first().is_none_or(|e| !e.is_parent()));
}

#[test]
fn single_click_directory_selects_without_navigation() {
    let mut panel = ResourcesPanel::new();
    panel.entries = vec![test_entry("subdir", EntryKind::Directory)];

    let opened_path = panel.handle_primary_click_on_entry(0, false, false);

    assert!(opened_path.is_none());
    assert!(panel.selected_indices.contains(&0));
    assert!(panel.navigation.is_at_root());
}

#[test]
fn single_click_parent_is_ignored() {
    let mut panel = ResourcesPanel::new();
    panel.navigation.push("subdir");
    panel.entries = vec![test_entry("..", EntryKind::Parent)];

    let opened_path = panel.handle_primary_click_on_entry(0, false, false);

    assert!(opened_path.is_none());
    assert!(panel.selected_indices.is_empty());
    assert_eq!(panel.navigation.depth(), 1);
}

#[test]
fn single_click_file_selects_without_navigation() {
    let mut panel = ResourcesPanel::new();
    panel.entries = vec![test_entry("player.lua", EntryKind::RegisteredFile)];

    let opened_path = panel.handle_primary_click_on_entry(0, false, false);

    assert!(opened_path.is_none());
    assert!(panel.selected_indices.contains(&0));
    assert!(panel.navigation.is_at_root());
}

#[test]
fn double_click_directory_navigates_and_clears_selection() {
    let mut panel = ResourcesPanel::new();
    panel.entries = vec![test_entry("subdir", EntryKind::Directory)];
    panel.selected_indices = [0_usize].into_iter().collect();

    let opened_path = panel.handle_primary_click_on_entry(0, false, true);

    assert!(opened_path.is_none());
    assert_eq!(panel.navigation.depth(), 1);
    assert_eq!(panel.navigation.segment(0), Some("subdir"));
    assert!(panel.selected_indices.is_empty());
}

#[test]
fn double_click_parent_navigates_up_and_clears_selection() {
    let mut panel = ResourcesPanel::new();
    panel.navigation.push("subdir");
    panel.entries = vec![test_entry("..", EntryKind::Parent)];
    panel.selected_indices = [0_usize].into_iter().collect();

    let opened_path = panel.handle_primary_click_on_entry(0, false, true);

    assert!(opened_path.is_none());
    assert!(panel.navigation.is_at_root());
    assert!(panel.selected_indices.is_empty());
}

#[test]
fn left_click_background_clears_selection() {
    let mut panel = ResourcesPanel::new();
    panel.selected_indices = [2_usize].into_iter().collect();

    panel.clear_selection();

    assert!(panel.selected_indices.is_empty());
}

#[test]
fn right_click_entry_selects_and_opens_context_menu() {
    let mut panel = ResourcesPanel::new();
    panel.entries = vec![test_entry("player.lua", EntryKind::RegisteredFile)];
    let click_pos = Vec2::new(32.0, 48.0);

    panel.handle_secondary_click_on_entry(0, click_pos);

    assert!(panel.selected_indices.contains(&0));
    match panel.active_menu.as_ref() {
        Some(ActiveMenu::Entry(target)) => {
            assert_eq!(target.entry_index, 0);
            assert_eq!(target.position, click_pos);
            assert_eq!(
                target.actions,
                vec![
                    ResourceMenuAction::Rename,
                    ResourceMenuAction::Delete,
                    ResourceMenuAction::Open,
                    ResourceMenuAction::Reveal,
                ]
            );
        }
        _ => panic!("expected entry menu"),
    }
}

#[test]
fn right_click_parent_does_not_select() {
    let mut panel = ResourcesPanel::new();
    panel.navigation.push("subdir");
    panel.entries = vec![test_entry("..", EntryKind::Parent)];
    let click_pos = Vec2::new(32.0, 48.0);

    panel.handle_secondary_click_on_entry(0, click_pos);

    assert!(panel.selected_indices.is_empty());
    assert!(panel.active_menu.is_none());
}

#[test]
fn shift_click_parent_does_not_select() {
    let mut panel = ResourcesPanel::new();
    panel.navigation.push("subdir");
    panel.entries = vec![test_entry("..", EntryKind::Parent)];

    let opened_path = panel.handle_primary_click_on_entry(0, true, false);

    assert!(opened_path.is_none());
    assert!(panel.selected_indices.is_empty());
}

#[test]
fn marquee_selection_excludes_parent_entry() {
    let mut panel = ResourcesPanel::new();
    panel.entries = vec![
        test_entry("..", EntryKind::Parent),
        test_entry("player.lua", EntryKind::RegisteredFile),
        test_entry("enemy.lua", EntryKind::RegisteredFile),
    ];

    panel.begin_marquee_selection(Vec2::new(0.0, 0.0), false);
    let filtered: BTreeSet<usize> = panel
        .entries
        .iter()
        .enumerate()
        .filter_map(|(index, entry)| {
            if entry.is_parent() {
                return None;
            }
            [0_usize, 1_usize, 2_usize]
                .contains(&index)
                .then_some(index)
        })
        .collect();
    panel.commit_marquee_selection(filtered);

    assert!(!panel.selected_indices.contains(&0));
    assert!(panel.selected_indices.contains(&1));
    assert!(panel.selected_indices.contains(&2));
}

#[test]
fn right_click_background_clears_selection_and_opens_context_menu() {
    let mut panel = ResourcesPanel::new();
    panel.selected_indices = [1_usize].into_iter().collect();
    let click_pos = Vec2::new(96.0, 128.0);

    panel.handle_secondary_click_on_background(click_pos);

    assert!(panel.selected_indices.is_empty());
    match panel.active_menu.as_ref() {
        Some(ActiveMenu::Background(pos)) => assert_eq!(*pos, click_pos),
        _ => panic!("expected background menu"),
    }
}

#[test]
fn parent_entry_appears_at_each_depth() {
    let (_test_game, _lock) = setup_test_game("resources_panel_parent_each_depth");
    let mut panel = ResourcesPanel::new();
    panel.navigation.push("subdir");
    panel.navigation.push("nested");
    panel.scan_current_dir(&AssetRegistry::default());

    let first = panel.entries.first().expect("should have parent entry");
    assert!(first.is_parent());

    panel.navigation.pop();
    panel.scan_current_dir(&AssetRegistry::default());
    let first = panel.entries.first().expect("should have parent entry");
    assert!(first.is_parent());
}

#[test]
fn registered_file_has_rename_delete_open_and_reveal_actions() {
    let entry = test_entry("registered.asset", EntryKind::RegisteredFile);

    assert_eq!(
        entry.context_menu_actions(),
        &[
            ResourceMenuAction::Rename,
            ResourceMenuAction::Delete,
            ResourceMenuAction::Open,
            ResourceMenuAction::Reveal,
        ]
    );
}

#[test]
fn unregistered_file_has_delete_open_and_reveal_actions() {
    let entry = test_entry("loose.asset", EntryKind::UnregisteredFile);

    assert_eq!(
        entry.context_menu_actions(),
        &[
            ResourceMenuAction::Delete,
            ResourceMenuAction::Open,
            ResourceMenuAction::Reveal,
        ]
    );
}

#[test]
fn directory_has_rename_delete_and_reveal_actions() {
    let entry = test_entry("folder", EntryKind::Directory);

    assert_eq!(
        entry.context_menu_actions(),
        &[
            ResourceMenuAction::Rename,
            ResourceMenuAction::Delete,
            ResourceMenuAction::Reveal,
        ]
    );
}

#[test]
fn system_directory_has_only_reveal_action() {
    let entry = test_entry("assets", EntryKind::SystemDirectory);

    assert_eq!(entry.context_menu_actions(), &[ResourceMenuAction::Reveal]);
}

#[test]
fn system_directory_is_dir_like() {
    let entry = test_entry("scripts", EntryKind::SystemDirectory);
    assert!(entry.is_dir_like());
    assert!(!entry.is_parent());
    assert!(!entry.is_registered());
}

#[test]
fn is_protected_path_detects_system_roots() {
    use engine_core::storage::system_folder::{is_protected_path, SYSTEM_FOLDER_ROOTS};

    let root = std::path::Path::new("/games/Demo/Resources");

    for &folder in SYSTEM_FOLDER_ROOTS {
        let path = root.join(folder);
        assert!(is_protected_path(&path, root), "should protect: {folder}");
    }
}

#[test]
fn is_protected_path_exact_match() {
    use engine_core::constants::paths;
    use engine_core::storage::system_folder::is_protected_path;

    let root = std::path::Path::new("/games/Demo/Resources");

    assert!(is_protected_path(&root.join(paths::SCRIPTS_FOLDER), root));
    assert!(is_protected_path(
        &root
            .join(paths::TEXT_FOLDER)
            .join(paths::TEXT_LANGUAGE_FOLDER)
            .join(paths::UI_TEXT_FOLDER),
        root
    ));
    assert!(is_protected_path(
        &root.join(paths::AUDIO_FOLDER).join(paths::SFX_FOLDER),
        root
    ));
    assert!(is_protected_path(
        &root.join(paths::AUDIO_FOLDER).join(paths::MUSIC_FOLDER),
        root
    ));
}

#[test]
fn is_protected_path_rejects_user_subdirs_inside_system_roots() {
    use engine_core::constants::paths;
    use engine_core::storage::system_folder::is_protected_path;

    let root = std::path::Path::new("/games/Demo/Resources");
    assert!(!is_protected_path(
        &root.join(paths::SCRIPTS_FOLDER).join("user_subdir"),
        root
    ));
    assert!(!is_protected_path(
        &root.join(paths::ASSETS_FOLDER).join("props"),
        root
    ));
    assert!(!is_protected_path(
        &root.join(paths::AUDIO_FOLDER).join("custom"),
        root
    ));
    assert!(!is_protected_path(
        &root.join(paths::TEXT_FOLDER).join("my_text"),
        root
    ));
}

#[test]
fn is_protected_path_rejects_user_dirs() {
    use engine_core::storage::system_folder::is_protected_path;

    let root = std::path::Path::new("/games/Demo/Resources");
    assert!(!is_protected_path(&root.join("my_stuff"), root));
    assert!(!is_protected_path(&root.join("user_data"), root));
}

#[test]
fn is_protected_path_rejects_root_itself() {
    use engine_core::storage::system_folder::is_protected_path;

    let root = std::path::Path::new("/games/Demo/Resources");
    assert!(!is_protected_path(root, root));
}

#[test]
fn background_menu_has_only_new_folder_action() {
    assert_eq!(
        context_menu::BACKGROUND_MENU_ACTIONS,
        &[ResourceMenuAction::NewFolder]
    );
}

#[test]
fn parent_entry_has_no_context_menu_actions() {
    let entry = test_entry("..", EntryKind::Parent);

    assert!(entry.context_menu_actions().is_empty());
}

#[test]
fn parent_entry_never_produces_a_context_target() {
    let entry = test_entry("..", EntryKind::Parent);
    assert!(context_target_for_entry(3, &entry, Vec2::new(10.0, 20.0)).is_none());
}

#[test]
fn regular_entry_context_target_keeps_index_position_and_actions() {
    let entry = test_entry("player.lua", EntryKind::RegisteredFile);
    let target = context_target_for_entry(7, &entry, Vec2::new(32.0, 48.0)).unwrap();

    assert_eq!(target.entry_index, 7);
    assert_eq!(target.position, Vec2::new(32.0, 48.0));
    assert_eq!(
        target.actions,
        vec![
            ResourceMenuAction::Rename,
            ResourceMenuAction::Delete,
            ResourceMenuAction::Open,
            ResourceMenuAction::Reveal,
        ]
    );
}

#[test]
fn active_menu_entry_stores_position() {
    let entry = test_entry("player.lua", EntryKind::RegisteredFile);
    let target = context_target_for_entry(2, &entry, Vec2::new(50.0, 75.0)).unwrap();
    let menu = ActiveMenu::Entry(target);
    match menu {
        ActiveMenu::Entry(t) => assert_eq!(t.position, Vec2::new(50.0, 75.0)),
        _ => panic!("expected Entry variant"),
    }
}

#[test]
fn active_menu_background_stores_position() {
    let menu = ActiveMenu::Background(Vec2::new(150.0, 200.0));
    match menu {
        ActiveMenu::Background(pos) => assert_eq!(pos, Vec2::new(150.0, 200.0)),
        _ => panic!("expected Background variant"),
    }
}

#[test]
fn pending_action_for_background_returns_create_directory() {
    let current_dir = PathBuf::from("/games/Demo/Resources/subdir");
    let action = context_menu::pending_action_for_background(&current_dir);
    assert!(matches!(action, PendingResourceAction::CreateDirectory(ref p) if p == &current_dir));
}

#[test]
fn open_resource_registered_prefab_returns_transition() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_game = TestGameFolder::new("open_resource_registered_prefab");
    let (mut editor, room_id, prefab_id, _root) = make_prefab_session_editor(&test_game);

    editor.prefab_editor = None;
    editor.mode = EditorMode::Room(room_id);

    let prefab_path = editor
        .game
        .asset_registry
        .record(AssetKey::Prefab(prefab_id))
        .map(|r| resources_folder_current().join(&r.path))
        .expect("prefab should be registered");

    let result = open_resource(&prefab_path, &mut editor);

    assert_eq!(result, ResourceOpenResult::PrefabTransition(prefab_id));
}

#[test]
fn open_resource_already_open_prefab_returns_transition() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_game = TestGameFolder::new("open_resource_already_open");
    let (mut editor, _room_id, prefab_id, _root) = make_prefab_session_editor(&test_game);

    let prefab_path = editor
        .game
        .asset_registry
        .record(AssetKey::Prefab(prefab_id))
        .map(|r| resources_folder_current().join(&r.path))
        .expect("prefab should be registered");

    editor.toast = None;

    let result = open_resource(&prefab_path, &mut editor);

    assert_eq!(result, ResourceOpenResult::PrefabTransition(prefab_id));
    assert!(editor.toast.is_none());
}

#[test]
fn resources_panel_multi_select_plain_click_replaces_previous_selection() {
    let mut panel = ResourcesPanel::new();
    panel.entries = vec![
        test_entry("scripts", EntryKind::Directory),
        test_entry("player.lua", EntryKind::RegisteredFile),
    ];
    panel.selected_indices = [0_usize].into_iter().collect();

    let opened_path = panel.handle_primary_click_on_entry(1, false, false);

    assert!(opened_path.is_none());
    assert_eq!(
        panel.selected_indices,
        [1_usize].into_iter().collect::<BTreeSet<_>>()
    );
}

#[test]
fn resources_panel_multi_select_shift_click_adds_unselected_entry() {
    let mut panel = ResourcesPanel::new();
    panel.entries = vec![
        test_entry("scripts", EntryKind::Directory),
        test_entry("player.lua", EntryKind::RegisteredFile),
    ];
    panel.selected_indices = [0_usize].into_iter().collect();

    let opened_path = panel.handle_primary_click_on_entry(1, true, false);

    assert!(opened_path.is_none());
    assert_eq!(
        panel.selected_indices,
        [0_usize, 1_usize].into_iter().collect::<BTreeSet<_>>()
    );
}

#[test]
fn resources_panel_multi_select_shift_click_toggles_selected_entry_off() {
    let mut panel = ResourcesPanel::new();
    panel.entries = vec![test_entry("player.lua", EntryKind::RegisteredFile)];
    panel.selected_indices = [0_usize].into_iter().collect();

    let opened_path = panel.handle_primary_click_on_entry(0, true, false);

    assert!(opened_path.is_none());
    assert!(panel.selected_indices.is_empty());
}

#[test]
fn resources_panel_multi_select_plain_marquee_replaces_selection() {
    let mut panel = ResourcesPanel::new();
    panel.selected_indices = [4_usize].into_iter().collect();

    panel.begin_marquee_selection(Vec2::new(10.0, 20.0), false);
    panel.commit_marquee_selection([1_usize, 3_usize].into_iter().collect());

    assert_eq!(
        panel.selected_indices,
        [1_usize, 3_usize].into_iter().collect::<BTreeSet<_>>()
    );
    assert!(!panel.marquee_selection.active);
    assert!(panel.marquee_selection.start_content_pos.is_none());
}

#[test]
fn resources_panel_multi_select_shift_marquee_toggles_matched_entries_off() {
    let mut panel = ResourcesPanel::new();
    panel.selected_indices = [0_usize, 4_usize].into_iter().collect();

    panel.begin_marquee_selection(Vec2::new(10.0, 20.0), true);
    panel.selected_indices.clear();
    panel.commit_marquee_selection([0_usize, 1_usize, 4_usize].into_iter().collect());

    assert_eq!(
        panel.selected_indices,
        [1_usize].into_iter().collect::<BTreeSet<_>>()
    );
}

#[test]
fn resources_panel_multi_select_shift_marquee_adds_new_entries() {
    let mut panel = ResourcesPanel::new();
    panel.selected_indices = [0_usize, 4_usize].into_iter().collect();

    panel.begin_marquee_selection(Vec2::new(10.0, 20.0), true);
    panel.selected_indices.clear();
    panel.commit_marquee_selection([1_usize, 3_usize].into_iter().collect());

    assert_eq!(
        panel.selected_indices,
        [0_usize, 1_usize, 3_usize, 4_usize]
            .into_iter()
            .collect::<BTreeSet<_>>()
    );
}

#[test]
fn resources_panel_multi_select_clear_selection_does_not_drop_marquee_snapshot() {
    let mut panel = ResourcesPanel::new();
    panel.selected_indices = [2_usize].into_iter().collect();

    panel.begin_marquee_selection(Vec2::new(5.0, 6.0), true);
    panel.clear_selection();
    panel.commit_marquee_selection(BTreeSet::new());

    assert_eq!(
        panel.selected_indices,
        [2_usize].into_iter().collect::<BTreeSet<_>>()
    );
}

#[test]
fn open_resource_unregistered_prefab_shows_toast() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_game = TestGameFolder::new("open_resource_unregistered_prefab");
    set_game_name(test_game.name());
    let mut editor = Editor {
        game: create_new_game(test_game.name().to_string()),
        ..Default::default()
    };

    let unregistered_path = resources_folder_current()
        .join(paths::PREFABS_FOLDER)
        .join(format!("ghost.{}", extensions::PREFAB));
    std::fs::create_dir_all(unregistered_path.parent().unwrap()).unwrap();
    std::fs::write(&unregistered_path, "").unwrap();

    let result = open_resource(&unregistered_path, &mut editor);

    assert_eq!(result, ResourceOpenResult::Handled);
    assert!(
        editor
            .toast
            .as_ref()
            .is_some_and(|t| t.msg == "Unregistered prefab file"),
        "expected toast for unregistered prefab, got: {}",
        editor
            .toast
            .as_ref()
            .map_or("None".to_string(), |t| t.msg.clone())
    );
}

#[test]
fn resources_panel_multi_select_content_space_position_accounts_for_scroll() {
    let content_rect = Rect::new(100.0, 200.0, 300.0, 400.0);
    let mouse = Vec2::new(160.0, 260.0);

    let pos = content_space_mouse_position(mouse, content_rect, -72.0);

    assert_eq!(pos, Vec2::new(60.0, 132.0));
}

#[test]
fn pending_delete_for_registered_file_returns_delete_registered_file() {
    let (_test_game, _lock) = setup_test_game("pending_delete_registered");
    let mut panel = ResourcesPanel::new();
    let mut registry = AssetRegistry::default();

    let full_path = resources_folder_current().join("scripts/test.lua");
    registry
        .register_asset_relative_path(AssetKey::Script(ScriptId(1)), "test.lua")
        .unwrap();
    registry.init_editor_metadata();

    panel.entries = vec![Entry {
        name: "test.lua".to_string(),
        display_name: "test.lua".to_string(),
        kind: EntryKind::RegisteredFile,
        path: full_path.clone(),
        icon_type: IconType::File,
    }];
    panel.selected_indices = [0_usize].into_iter().collect();

    let action = panel.pending_delete_for_selection(&registry);
    assert!(matches!(
        action,
        Some(PendingResourceAction::DeleteRegisteredFile(
            AssetKey::Script(ScriptId(1))
        ))
    ));
}

#[test]
fn pending_delete_for_unregistered_file_returns_delete_unregistered_file() {
    let mut panel = ResourcesPanel::new();
    let registry = AssetRegistry::default();

    let path = PathBuf::from("loose.txt");
    panel.entries = vec![Entry {
        name: "loose.txt".to_string(),
        display_name: "loose.txt".to_string(),
        kind: EntryKind::UnregisteredFile,
        path: path.clone(),
        icon_type: IconType::File,
    }];
    panel.selected_indices = [0_usize].into_iter().collect();

    let action = panel.pending_delete_for_selection(&registry);
    assert!(
        matches!(action, Some(PendingResourceAction::DeleteUnregisteredFile(ref p)) if p == &path)
    );
}

#[test]
fn pending_delete_for_directory_returns_delete_directory() {
    let mut panel = ResourcesPanel::new();
    let registry = AssetRegistry::default();

    let path = PathBuf::from("my_folder");
    panel.entries = vec![Entry {
        name: "my_folder".to_string(),
        display_name: "my_folder".to_string(),
        kind: EntryKind::Directory,
        path: path.clone(),
        icon_type: IconType::Folder,
    }];
    panel.selected_indices = [0_usize].into_iter().collect();

    let action = panel.pending_delete_for_selection(&registry);
    assert!(
        matches!(action, Some(PendingResourceAction::DeleteDirectory(ref p)) if p.as_ref() == path.as_path())
    );
}

#[test]
fn pending_delete_for_parent_returns_none() {
    let mut panel = ResourcesPanel::new();
    let registry = AssetRegistry::default();

    panel.entries = vec![test_entry("..", EntryKind::Parent)];
    panel.selected_indices = [0_usize].into_iter().collect();

    assert!(panel.pending_delete_for_selection(&registry).is_none());
}

#[test]
fn pending_delete_for_system_directory_returns_none() {
    let mut panel = ResourcesPanel::new();
    let registry = AssetRegistry::default();

    panel.entries = vec![test_entry("assets", EntryKind::SystemDirectory)];
    panel.selected_indices = [0_usize].into_iter().collect();

    assert!(panel.pending_delete_for_selection(&registry).is_none());
}

#[test]
fn pending_delete_for_no_selection_returns_none() {
    let mut panel = ResourcesPanel::new();
    let registry = AssetRegistry::default();

    panel.entries = vec![test_entry("file.lua", EntryKind::RegisteredFile)];
    panel.selected_indices.clear();

    assert!(panel.pending_delete_for_selection(&registry).is_none());
}

#[test]
fn pending_delete_for_multi_selection_returns_none() {
    let mut panel = ResourcesPanel::new();
    let registry = AssetRegistry::default();

    panel.entries = vec![
        test_entry("a.lua", EntryKind::RegisteredFile),
        test_entry("b.lua", EntryKind::RegisteredFile),
    ];
    panel.selected_indices = [0_usize, 1_usize].into_iter().collect();

    assert!(panel.pending_delete_for_selection(&registry).is_none());
}

#[test]
fn resources_panel_multi_select_right_click_collapses_to_clicked_entry() {
    let mut panel = ResourcesPanel::new();
    panel.entries = vec![
        test_entry("scripts", EntryKind::Directory),
        test_entry("player.lua", EntryKind::RegisteredFile),
        test_entry("enemy.lua", EntryKind::RegisteredFile),
    ];
    panel.selected_indices = [0_usize, 2_usize].into_iter().collect();

    panel.handle_secondary_click_on_entry(1, Vec2::new(32.0, 48.0));

    assert_eq!(
        panel.selected_indices,
        [1_usize].into_iter().collect::<BTreeSet<_>>()
    );
    match panel.active_menu.as_ref() {
        Some(ActiveMenu::Entry(target)) => assert_eq!(target.entry_index, 1),
        _ => panic!("expected entry menu"),
    }
}

#[test]
fn resources_panel_multi_select_double_click_directory_clears_selection() {
    let mut panel = ResourcesPanel::new();
    panel.entries = vec![test_entry("subdir", EntryKind::Directory)];
    panel.selected_indices = [0_usize, 3_usize].into_iter().collect();

    let opened_path = panel.handle_primary_click_on_entry(0, false, true);

    assert!(opened_path.is_none());
    assert!(panel.selected_indices.is_empty());
    assert_eq!(panel.navigation.segment(0), Some("subdir"));
}
