use super::*;

#[test]
fn revert_instance_to_prefab_command_clears_overrides_and_restores_prefab_state() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_game = TestGameFolder::new("prefab_revert_instance_to_prefab");
    set_game_name(test_game.name());
    let (mut editor, room_id, prefab_id, _) = make_prefab_session_editor(&test_game);
    editor.close_active_prefab_editor();

    let linked_root = linked_root_entities(&editor.game.ecs, prefab_id)[0];
    editor
        .game
        .ecs
        .get_mut::<Name>(linked_root)
        .expect("linked root should have name")
        .0 = "Edited Root".to_string();
    editor.game.ecs.add_component_to_entity(
        linked_root,
        PrefabOverrides {
            modified_components: vec![Name::TYPE_NAME.to_string()],
            ..Default::default()
        },
    );
    editor.room_editor.set_selected_entity(Some(linked_root));

    let _services = EditorServicesGuard::install(editor);

    push_command(Box::new(RevertPrefabInstanceCmd::new(
        linked_root,
        EditorMode::Room(room_id),
    )));
    apply_pending_commands();

    with_editor(|editor| {
        assert_eq!(
            editor
                .game
                .ecs
                .get::<Name>(linked_root)
                .map(|name| name.0.as_str()),
            Some("Root")
        );
        assert!(!editor.game.ecs.has::<PrefabOverrides>(linked_root));
        assert_eq!(
            editor.room_editor.single_selected_entity(),
            Some(linked_root)
        );
    });

    request_undo();
    apply_pending_commands();

    with_editor(|editor| {
        assert_eq!(
            editor
                .game
                .ecs
                .get::<Name>(linked_root)
                .map(|name| name.0.as_str()),
            Some("Edited Root")
        );
        assert!(editor.game.ecs.has::<PrefabOverrides>(linked_root));
    });
}
