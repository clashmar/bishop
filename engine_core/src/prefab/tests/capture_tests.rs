use super::*;
use crate::prelude::{set_parent, ClipId, SpriteId};

#[test]
fn capture_prefab_normalizes_root_offset_and_instantiate_restores_world_positions() {
    let mut game = test_game();
    let room_id = RoomId(7);

    let root = game
        .ecs
        .create_entity()
        .with(Name("Root".to_string()))
        .with(Transform {
            position: Vec2::new(10.0, 15.0),
            ..Default::default()
        })
        .with(CurrentRoom(room_id))
        .finish();
    let child = game
        .ecs
        .create_entity()
        .with(Name("Child".to_string()))
        .with(Transform {
            position: Vec2::new(14.0, 18.0),
            ..Default::default()
        })
        .finish();
    set_parent(&mut game.ecs, child, root);

    let prefab = capture_prefab(&mut game.ecs, root, PrefabId(1), "crate".to_string());

    let saved_root = prefab
        .nodes
        .iter()
        .find(|node| node.node_id == prefab.root_node_id)
        .unwrap();
    let saved_child = prefab
        .nodes
        .iter()
        .find(|node| node.node_id != prefab.root_node_id)
        .unwrap();
    let root_transform = saved_root
        .components
        .iter()
        .find(|component| component.type_name == comp_type_name::<Transform>())
        .unwrap();
    let child_transform = saved_child
        .components
        .iter()
        .find(|component| component.type_name == comp_type_name::<Transform>())
        .unwrap();

    assert!(root_transform.ron.contains("position:(0.0,0.0)"));
    assert!(child_transform.ron.contains("position:(4.0,3.0)"));
    assert!(!saved_root
        .components
        .iter()
        .any(|component| component.type_name == comp_type_name::<CurrentRoom>()));

    let root_entity = {
        let mut ctx = game.ctx_mut();
        instantiate_prefab(&mut ctx, &prefab, Vec2::new(100.0, 200.0), Some(room_id))
    };

    let child_entity = crate::ecs::entity::get_children(&game.ecs, root_entity)
        .into_iter()
        .next()
        .unwrap();
    let instantiated_root = game.ecs.get::<Transform>(root_entity).unwrap();
    let instantiated_child = game.ecs.get::<Transform>(child_entity).unwrap();

    assert_eq!(instantiated_root.position, Vec2::new(100.0, 200.0));
    assert_eq!(instantiated_child.position, Vec2::new(104.0, 203.0));
    assert_eq!(
        game.ecs.get::<CurrentRoom>(root_entity).map(|room| room.0),
        Some(room_id)
    );
    assert_eq!(
        game.ecs.get::<CurrentRoom>(child_entity).map(|room| room.0),
        Some(room_id)
    );
}

#[test]
fn capture_prefab_excludes_runtime_current_frame_components() {
    let mut game = test_game();

    let root = game
        .ecs
        .create_entity()
        .with(Name("Animated Root".to_string()))
        .with(Transform::default())
        .with(CurrentFrame {
            clip_id: ClipId::Idle,
            col: 2,
            row: 1,
            offset: Vec2::new(3.0, 4.0),
            sprite_id: SpriteId(7),
            frame_size: Vec2::new(16.0, 16.0),
            flip_x: false,
        })
        .finish();

    let prefab = capture_prefab(&mut game.ecs, root, PrefabId(1), "crate".to_string());
    let saved_root = prefab
        .nodes
        .iter()
        .find(|node| node.node_id == prefab.root_node_id)
        .unwrap();

    assert!(!saved_root
        .components
        .iter()
        .any(|component| component.type_name == comp_type_name::<CurrentFrame>()));
}

#[test]
fn capture_prefab_with_existing_preserves_stable_node_ids() {
    let mut game = test_game();
    let prefab = PrefabAsset {
        id: PrefabId(1),
        name: "crate".to_string(),
        next_node_id: 10,
        root_node_id: 4,
        nodes: vec![
            PrefabNode {
                node_id: 4,
                parent_node_id: None,
                components: vec![ComponentSnapshot {
                    type_name: comp_type_name::<Name>().to_string(),
                    ron: "(\"Root\")".to_string(),
                }],
            },
            PrefabNode {
                node_id: 7,
                parent_node_id: Some(4),
                components: vec![ComponentSnapshot {
                    type_name: comp_type_name::<Name>().to_string(),
                    ron: "(\"Child\")".to_string(),
                }],
            },
        ],
    };

    let root_entity = {
        let mut ctx = game.ctx_mut();
        instantiate_prefab(&mut ctx, &prefab, Vec2::ZERO, None)
    };
    let extra_child = game
        .ecs
        .create_entity()
        .with(Name("Extra".to_string()))
        .finish();
    set_parent(&mut game.ecs, extra_child, root_entity);

    let captured = capture_prefab_with_existing(
        &mut game.ecs,
        root_entity,
        prefab.id,
        "crate".to_string(),
        Some(&prefab),
    );
    let node_ids = captured
        .nodes
        .iter()
        .map(|node| node.node_id)
        .collect::<HashSet<_>>();

    assert!(node_ids.contains(&4));
    assert!(node_ids.contains(&7));
    assert!(node_ids.contains(&10));
    assert_eq!(captured.next_node_id, 11);
}
