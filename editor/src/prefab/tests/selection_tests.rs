use super::*;

#[test]
fn creating_entity_replaces_stale_root_with_new_root() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_game = TestGameFolder::new("prefab_stale_root");
    set_game_name(test_game.name());
    let mut editor = PrefabEditor::new(
        PrefabId(1),
        "Prefab".to_string(),
        StagedPrefabState::Empty,
        PrefabRoomSyncState {
            staged_prefab: StagedPrefabState::Empty,
            linked_instance_snapshots: Vec::new(),
        },
    );
    let mut stage = PrefabStage::new(test_game.name());

    let stale_root = stage
        .ecs
        .create_entity()
        .with(Transform::default())
        .with(Name("Old Root".to_string()))
        .finish();
    editor.root_entity = Some(stale_root);
    editor.set_selected_entity(Some(stale_root));

    {
        let mut ctx = stage.ctx_mut();
        Ecs::remove_entity(&mut ctx, stale_root);
    }

    let new_entity = editor.create_prefab_entity(&mut stage.ecs, None);

    assert_eq!(editor.root_entity, Some(new_entity));
    assert_eq!(get_parent(&stage.ecs, new_entity), None);
}
