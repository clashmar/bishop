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
