use super::*;

#[test]
fn apply_instance_to_prefab_command_updates_other_linked_instances_and_supports_undo() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_game = TestGameFolder::new("prefab_apply_instance_to_prefab");
    set_game_name(test_game.name());
    let (mut editor, room_id, prefab_id, _) = make_prefab_session_editor(&test_game);
    editor.close_active_prefab_editor();

    let linked_root = linked_root_entities(&editor.game.ecs, prefab_id)[0];
    let clean_sibling_root = {
        let prefab = editor
            .game
            .prefab_library
            .prefabs
            .get(&prefab_id)
            .cloned()
            .expect("prefab should exist");
        let mut ctx = editor.game.ctx_mut();
        let mut services = ctx.services_ctx_mut();
        instantiate_prefab(
            &mut services,
            &prefab,
            Vec2::new(160.0, 96.0),
            Some(room_id),
        )
    };
    let overridden_sibling_root = {
        let prefab = editor
            .game
            .prefab_library
            .prefabs
            .get(&prefab_id)
            .cloned()
            .expect("prefab should exist");
        let mut ctx = editor.game.ctx_mut();
        let mut services = ctx.services_ctx_mut();
        instantiate_prefab(
            &mut services,
            &prefab,
            Vec2::new(256.0, 96.0),
            Some(room_id),
        )
    };
    editor
        .game
        .ecs
        .get_mut::<Name>(linked_root)
        .expect("linked root should have a name")
        .0 = "Edited Root".to_string();
    editor
        .game
        .ecs
        .get_mut::<Name>(overridden_sibling_root)
        .expect("overridden sibling should have a name")
        .0 = "Locally Tweaked Child".to_string();
    editor.game.ecs.add_component_to_entity(
        overridden_sibling_root,
        PrefabOverrides {
            modified_components: vec![Name::TYPE_NAME.to_string()],
            ..Default::default()
        },
    );

    let _services = EditorServicesGuard::install(editor);

    push_command(Box::new(ApplyInstanceToPrefabCmd::new(
        linked_root,
        EditorMode::Room(room_id),
    )));
    apply_pending_commands();

    with_editor(|editor| {
        let prefab = editor
            .game
            .prefab_library
            .prefabs
            .get(&prefab_id)
            .expect("prefab should be updated");
        let prefab_root = prefab
            .nodes
            .iter()
            .find(|node| node.node_id == prefab.root_node_id)
            .expect("prefab root node should exist");
        assert!(prefab_root
            .components
            .iter()
            .any(|component| component.type_name == Name::TYPE_NAME
                && component.ron.contains("Edited Root")));
        assert_eq!(
            editor
                .game
                .ecs
                .get::<Name>(clean_sibling_root)
                .map(|name| name.0.as_str()),
            Some("Edited Root")
        );
        assert_eq!(
            editor
                .game
                .ecs
                .get::<Name>(overridden_sibling_root)
                .map(|name| name.0.as_str()),
            Some("Locally Tweaked Child")
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
    });
}
