use super::*;

fn prefab_library(ids: &[usize]) -> PrefabLibrary {
    let mut library = PrefabLibrary::default();
    for id in ids {
        let prefab_id = PrefabId(*id);
        library.prefabs.insert(
            prefab_id,
            PrefabAsset {
                id: prefab_id,
                name: format!("Prefab {id}"),
                next_node_id: 2,
                root_node_id: 1,
                nodes: vec![PrefabNode {
                    node_id: 1,
                    parent_node_id: None,
                    components: vec![],
                }],
            },
        );
    }
    library
}

#[test]
fn recent_prefabs_are_newest_first_deduped_and_capped_at_ten() {
    let mut editor = RoomEditor::new();

    for prefab_id in [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 3] {
        editor.record_recent_prefab(PrefabId(prefab_id));
    }

    assert_eq!(
        editor.recent_prefab_ids,
        vec![
            PrefabId(3),
            PrefabId(12),
            PrefabId(11),
            PrefabId(10),
            PrefabId(9),
            PrefabId(8),
            PrefabId(7),
            PrefabId(6),
            PrefabId(5),
            PrefabId(4),
        ]
    );
}

#[test]
fn loading_palette_state_restores_active_prefab_without_restoring_stamp_mode() {
    let mut editor = RoomEditor::new();
    editor.scene_sub_mode = RoomSceneSubMode::Stamp;

    editor.load_prefab_palette_state(
        &prefab_library(&[2, 4, 6, 8, 10, 12]),
        crate::storage::editor_storage::PrefabPaletteState {
            active_prefab_id: Some(PrefabId(4)),
            recent_prefab_ids: vec![
                PrefabId(12),
                PrefabId(10),
                PrefabId(8),
                PrefabId(6),
                PrefabId(4),
                PrefabId(2),
            ],
        },
    );

    assert_eq!(editor.active_prefab_id, Some(PrefabId(4)));
    assert_eq!(
        editor.recent_prefab_ids,
        vec![
            PrefabId(12),
            PrefabId(10),
            PrefabId(8),
            PrefabId(6),
            PrefabId(4),
            PrefabId(2),
        ]
    );
    assert_eq!(editor.scene_sub_mode, RoomSceneSubMode::Scene);
}

#[test]
fn loading_palette_state_filters_missing_prefabs_and_caps_recent_entries() {
    let mut editor = RoomEditor::new();

    editor.load_prefab_palette_state(
        &prefab_library(&[1, 3, 5, 7, 9, 11, 13, 15, 17, 19, 21, 23]),
        crate::storage::editor_storage::PrefabPaletteState {
            active_prefab_id: Some(PrefabId(999)),
            recent_prefab_ids: vec![
                PrefabId(999),
                PrefabId(23),
                PrefabId(21),
                PrefabId(19),
                PrefabId(17),
                PrefabId(15),
                PrefabId(13),
                PrefabId(11),
                PrefabId(9),
                PrefabId(7),
                PrefabId(5),
                PrefabId(3),
                PrefabId(1),
            ],
        },
    );

    assert_eq!(editor.active_prefab_id, None);
    assert_eq!(
        editor.recent_prefab_ids,
        vec![
            PrefabId(23),
            PrefabId(21),
            PrefabId(19),
            PrefabId(17),
            PrefabId(15),
            PrefabId(13),
            PrefabId(11),
            PrefabId(9),
            PrefabId(7),
            PrefabId(5),
        ]
    );
}

#[test]
fn reconcile_prefab_palette_promotes_first_valid_recent_when_active_is_missing() {
    let library = prefab_library(&[2, 3]);
    let mut editor = RoomEditor::new();
    editor.active_prefab_id = Some(PrefabId(999));
    editor.recent_prefab_ids = vec![PrefabId(999), PrefabId(2), PrefabId(3)];
    editor.mode = RoomEditorMode::Tilemap;
    editor.scene_sub_mode = RoomSceneSubMode::Stamp;
    editor.view_preview = true;
    editor.preview_camera_id = Some(7);

    editor.reconcile_prefab_palette(&library);

    assert_eq!(editor.active_prefab_id, Some(PrefabId(2)));
    assert_eq!(editor.recent_prefab_ids, vec![PrefabId(2), PrefabId(3)]);
    assert_eq!(editor.mode, RoomEditorMode::Tilemap);
    assert_eq!(editor.scene_sub_mode, RoomSceneSubMode::Stamp);
    assert!(editor.view_preview);
    assert_eq!(editor.preview_camera_id, Some(7));
}
