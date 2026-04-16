use super::*;

#[test]
fn room_component_edits_write_prefab_overrides_for_linked_instances() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_game = TestGameFolder::new("prefab_component_override_tracking");
    set_game_name(test_game.name());
    let (mut editor, room_id, prefab_id, _) = make_prefab_session_editor(&test_game);
    editor.close_active_prefab_editor();

    let linked_root = linked_root_entities(&editor.game.ecs, prefab_id)[0];
    let old_ron = ron::to_string(
        &editor
            .game
            .ecs
            .get::<Name>(linked_root)
            .expect("linked instance should have a name"),
    )
    .expect("name should serialize");

    let _services = EditorServicesGuard::install(editor);

    push_command(Box::new(UpdateComponentCmd::new(
        linked_root,
        EditorMode::Room(room_id),
        Name::TYPE_NAME,
        old_ron,
        "(\"Edited Root\")".to_string(),
        Default::default(),
        Default::default(),
    )));
    apply_pending_commands();

    with_editor(|editor| {
        let overrides = editor
            .game
            .ecs
            .get::<PrefabOverrides>(linked_root)
            .expect("linked instance edit should create prefab overrides");
        assert!(overrides
            .modified_components
            .iter()
            .any(|type_name| type_name == Name::TYPE_NAME));
    });
}
