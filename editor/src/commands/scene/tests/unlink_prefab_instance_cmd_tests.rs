use super::*;

#[test]
fn unlink_prefab_instance_command_clears_prefab_components_and_restores_on_undo() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_game = TestGameFolder::new("prefab_unlink_instance");
    set_game_name(test_game.name());
    let (mut editor, room_id, prefab_id, _) = make_prefab_session_editor(&test_game);
    editor.close_active_prefab_editor();
    let linked_root = linked_root_entities(&editor.game.ecs, prefab_id)[0];

    let _services = EditorServicesGuard::install(editor);

    push_command(Box::new(UnlinkPrefabInstanceCmd::new(
        linked_root,
        EditorMode::Room(room_id),
    )));
    apply_pending_commands();

    with_editor(|editor| {
        assert!(editor.game.ecs.has::<Transform>(linked_root));
        assert!(!editor.game.ecs.has::<PrefabInstanceRoot>(linked_root));
        assert_eq!(linked_root_entities(&editor.game.ecs, prefab_id).len(), 0);
    });

    request_undo();
    apply_pending_commands();

    with_editor(|editor| {
        assert_eq!(linked_root_entities(&editor.game.ecs, prefab_id).len(), 1);
        assert_eq!(
            editor
                .game
                .ecs
                .get::<PrefabInstanceRoot>(linked_root)
                .map(|root| root.prefab_id),
            Some(prefab_id)
        );
    });
}
