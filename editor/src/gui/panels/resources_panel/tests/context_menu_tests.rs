use super::super::*;
use super::*;

use super::super::context_menu::{
    self, context_target_for_entry, ActiveMenu, EntryKind, PendingResourceAction,
    ResourceMenuAction,
};
use std::collections::BTreeSet;

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
fn registered_file_has_rename_delete_open_and_reveal_actions() {
    let entry = test_entry("registered.asset", EntryKind::RegisteredFile);

    assert_eq!(
        entry.context_menu_actions(),
        vec![
            ResourceMenuAction::Rename,
            ResourceMenuAction::Delete,
            ResourceMenuAction::Open,
            ResourceMenuAction::Reveal,
        ]
    );
}

#[test]
fn unregistered_file_in_canonical_folder_has_register_delete_open_and_reveal_actions() {
    let file_name = format!("loose.{}", engine_core::constants::extensions::LUA);
    let mut entry = test_entry(&file_name, EntryKind::UnregisteredFile);
    entry.path = engine_core::storage::path_utils::resources_folder_current()
        .join(engine_core::constants::paths::SCRIPTS_FOLDER)
        .join(&file_name);

    assert_eq!(
        entry.context_menu_actions(),
        vec![
            ResourceMenuAction::Register,
            ResourceMenuAction::Delete,
            ResourceMenuAction::Open,
            ResourceMenuAction::Reveal,
        ]
    );
}

#[test]
fn register_action_infers_font_kind_from_supported_extension() {
    let font_path = format!("display.{}", engine_core::constants::extensions::OTF);
    let mut entry = test_entry(&font_path, EntryKind::UnregisteredFile);
    let expected_path = engine_core::storage::path_utils::fonts_folder().join(&font_path);
    entry.path = expected_path.clone();
    let action = context_menu::pending_action_for(
        &entry,
        ResourceMenuAction::Register,
        &AssetRegistry::default(),
    );

    assert!(matches!(
        action,
        Some(PendingResourceAction::Register(AssetKind::Font, ref path))
            if path == &expected_path
    ));
}

#[test]
fn unsupported_file_does_not_offer_register_action() {
    let entry = test_entry("unknown.woff", EntryKind::UnregisteredFile);
    let target = context_target_for_entry(0, &entry, Vec2::new(0.0, 0.0)).unwrap();
    assert!(!target.actions.contains(&ResourceMenuAction::Register));
}

#[test]
fn resources_root_icon_does_not_offer_register_action() {
    let mut entry = test_entry("Icon.png", EntryKind::UnregisteredFile);
    entry.path = engine_core::storage::path_utils::resources_folder_current().join("Icon.png");
    let target = context_target_for_entry(0, &entry, Vec2::new(0.0, 0.0)).unwrap();
    assert!(!target.actions.contains(&ResourceMenuAction::Register));
}

#[test]
fn directory_has_rename_delete_and_reveal_actions() {
    let entry = test_entry("folder", EntryKind::Directory);

    assert_eq!(
        entry.context_menu_actions(),
        vec![
            ResourceMenuAction::Rename,
            ResourceMenuAction::Delete,
            ResourceMenuAction::Reveal,
        ]
    );
}

#[test]
fn system_directory_has_only_reveal_action() {
    let entry = test_entry("assets", EntryKind::SystemDirectory);

    assert_eq!(entry.context_menu_actions(), vec![ResourceMenuAction::Reveal]);
}

#[test]
fn system_directory_is_dir_like() {
    let entry = test_entry("scripts", EntryKind::SystemDirectory);
    assert!(entry.is_dir_like());
    assert!(!entry.is_parent());
    assert!(!entry.is_registered());
}

#[test]
fn background_menu_has_one_generic_import_action() {
    assert_eq!(
        context_menu::BACKGROUND_MENU_ACTIONS,
        &[ResourceMenuAction::NewFolder, ResourceMenuAction::Import]
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
    let resources_root = engine_core::storage::path_utils::resources_folder_current();
    let current_dir = resources_root.join("subdir");
    let action = context_menu::pending_action_for_background(
        &current_dir,
        &resources_root,
        ResourceMenuAction::NewFolder,
    );
    assert!(matches!(action, Some(PendingResourceAction::CreateDirectory(ref p)) if p == &current_dir));
}

#[test]
fn resources_root_background_menu_hides_import() {
    let resources_root = engine_core::storage::path_utils::resources_folder_current();
    assert_eq!(
        context_menu::background_menu_actions_for(&resources_root, &resources_root),
        &[ResourceMenuAction::NewFolder]
    );
    assert!(context_menu::pending_action_for_background(
        &resources_root,
        &resources_root,
        ResourceMenuAction::Import,
    )
    .is_none());
}

#[test]
fn import_from_asset_directory_is_constrained_to_sprite_kind() {
    let resources_root = engine_core::storage::path_utils::resources_folder_current();
    let current_dir = resources_root.join(engine_core::constants::paths::ASSETS_FOLDER);
    let action = context_menu::pending_action_for_background(
        &current_dir,
        &resources_root,
        ResourceMenuAction::Import,
    );

    assert!(matches!(
        action,
        Some(PendingResourceAction::Import {
            expected_kind: Some(AssetKind::Sprite),
            current_dir: ref path,
        }) if path == &current_dir
    ));
}

#[test]
fn import_from_audio_subdirectory_is_constrained_to_sound_kind() {
    let resources_root = engine_core::storage::path_utils::resources_folder_current();
    let current_dir = resources_root
        .join(engine_core::constants::paths::AUDIO_FOLDER)
        .join(engine_core::constants::paths::SFX_FOLDER);
    let action = context_menu::pending_action_for_background(
        &current_dir,
        &resources_root,
        ResourceMenuAction::Import,
    );

    assert!(matches!(
        action,
        Some(PendingResourceAction::Import {
            expected_kind: Some(AssetKind::Sound),
            current_dir: ref path,
        }) if path == &current_dir
    ));
}

#[test]
fn import_from_user_directory_allows_any_supported_kind() {
    let resources_root = engine_core::storage::path_utils::resources_folder_current();
    let current_dir = resources_root.join("MyCustomFolder");
    assert_eq!(
        context_menu::background_menu_actions_for(&current_dir, &resources_root),
        context_menu::BACKGROUND_MENU_ACTIONS
    );
    let action = context_menu::pending_action_for_background(
        &current_dir,
        &resources_root,
        ResourceMenuAction::Import,
    );

    assert!(matches!(
        action,
        Some(PendingResourceAction::Import {
            expected_kind: None,
            current_dir: ref path,
        }) if path == &current_dir
    ));
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
