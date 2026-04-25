use super::*;
use crate::room::selection::selection_render_rect;

fn prefab_manager(ids: &[usize]) -> PrefabManager {
    let mut manager = PrefabManager::default();
    for id in ids {
        let prefab_id = PrefabId(*id);
        manager.prefabs.insert(
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
    manager
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
        &prefab_manager(&[2, 4, 6, 8, 10, 12]),
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
        &prefab_manager(&[1, 3, 5, 7, 9, 11, 13, 15, 17, 19, 21, 23]),
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
    let manager = prefab_manager(&[2, 3]);
    let mut editor = RoomEditor::new();
    editor.active_prefab_id = Some(PrefabId(999));
    editor.recent_prefab_ids = vec![PrefabId(999), PrefabId(2), PrefabId(3)];
    editor.mode = RoomEditorMode::Tilemap;
    editor.scene_sub_mode = RoomSceneSubMode::Stamp;
    editor.view_preview = true;
    editor.preview_camera_id = Some(7);

    editor.reconcile_prefab_palette(&manager);

    assert_eq!(editor.active_prefab_id, Some(PrefabId(2)));
    assert_eq!(editor.recent_prefab_ids, vec![PrefabId(2), PrefabId(3)]);
    assert_eq!(editor.mode, RoomEditorMode::Tilemap);
    assert_eq!(editor.scene_sub_mode, RoomSceneSubMode::Stamp);
    assert!(editor.view_preview);
    assert_eq!(editor.preview_camera_id, Some(7));
}

#[test]
fn odd_width_bottom_center_sprite_world_rect_matches_static_sprite_draw() {
    let (top_left, size) = selection_render_rect(
        Vec2::ZERO,
        8.0,
        Pivot::BottomCenter,
        false,
        Some(vec2(5.0, 5.0)),
        None,
    );

    assert_eq!(top_left, vec2(-2.5, -5.0));
    assert_eq!(size, vec2(5.0, 5.0));
}

#[test]
fn even_width_bottom_center_sprite_world_rect_keeps_existing_alignment() {
    let (top_left, size) = selection_render_rect(
        Vec2::ZERO,
        8.0,
        Pivot::BottomCenter,
        false,
        Some(vec2(24.0, 24.0)),
        None,
    );

    assert_eq!(top_left, vec2(-12.0, -24.0));
    assert_eq!(size, vec2(24.0, 24.0));
}

#[test]
fn static_sprite_branch_matches_sprite_size() {
    let (top_left, size) = selection_render_rect(
        Vec2::ZERO,
        8.0,
        Pivot::BottomCenter,
        false,
        Some(vec2(5.0, 5.0)),
        None,
    );

    assert_eq!(top_left, vec2(-2.5, -5.0));
    assert_eq!(size, vec2(5.0, 5.0));
}

#[test]
fn current_frame_selection_uses_offset_without_snapping() {
    let (top_left, size) = selection_render_rect(
        Vec2::ZERO,
        8.0,
        Pivot::BottomCenter,
        false,
        Some(vec2(5.0, 5.0)),
        Some(&CurrentFrame {
            sprite_id: SpriteId(2),
            frame_size: vec2(5.0, 5.0),
            offset: vec2(0.25, -0.75),
            ..Default::default()
        }),
    );

    assert_eq!(top_left, vec2(-2.25, -5.75));
    assert_eq!(size, vec2(5.0, 5.0));
}

#[test]
fn stale_current_frame_uses_static_sprite_fallback_size_and_position() {
    let (top_left, size) = selection_render_rect(
        Vec2::ZERO,
        8.0,
        Pivot::BottomCenter,
        false,
        Some(vec2(5.0, 5.0)),
        Some(&CurrentFrame {
            sprite_id: SpriteId(0),
            frame_size: vec2(9.0, 9.0),
            offset: vec2(4.5, 4.5),
            ..Default::default()
        }),
    );

    assert_eq!(top_left, vec2(-2.5, -5.0));
    assert_eq!(size, vec2(5.0, 5.0));
}

#[test]
fn unset_sprite_keeps_unsnapped_fallback_alignment() {
    let (top_left, size) =
        selection_render_rect(Vec2::ZERO, 8.0, Pivot::BottomCenter, false, None, None);

    assert_eq!(top_left, vec2(-4.0, -8.0));
    assert_eq!(size, vec2(8.0, 8.0));
}

#[test]
fn placeholder_selection_keeps_grid_centering_behavior() {
    let (top_left, size) = selection_render_rect(
        Vec2::ZERO,
        8.0,
        Pivot::BottomCenter,
        true,
        Some(vec2(5.0, 5.0)),
        None,
    );

    assert_eq!(top_left, vec2(-4.0, -4.0));
    assert_eq!(size, vec2(8.0, 8.0));
}
