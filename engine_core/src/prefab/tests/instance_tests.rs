use super::*;

#[test]
fn refresh_prefab_instance_preserves_added_local_components() {
    let mut game = test_game();
    let prefab = PrefabAsset {
        id: PrefabId(1),
        name: "crate".to_string(),
        next_node_id: 2,
        root_node_id: 1,
        nodes: vec![PrefabNode {
            node_id: 1,
            parent_node_id: None,
            components: vec![ComponentSnapshot {
                type_name: comp_type_name::<Name>().to_string(),
                ron: "(\"Prefab Root\")".to_string(),
            }],
        }],
    };

    let root_entity = {
        let mut ctx = game.ctx_mut();
        instantiate_prefab(&mut ctx, &prefab, Vec2::ZERO, None)
    };

    game.ecs
        .add_component_to_entity(root_entity, Velocity { x: 2.0, y: 4.0 });
    game.ecs.add_component_to_entity(
        root_entity,
        PrefabOverrides {
            added_components: vec![ComponentSnapshot {
                type_name: comp_type_name::<Velocity>().to_string(),
                ron: "(x:2.0,y:4.0)".to_string(),
            }],
            ..Default::default()
        },
    );

    let updated_prefab = PrefabAsset {
        nodes: vec![PrefabNode {
            node_id: 1,
            parent_node_id: None,
            components: vec![ComponentSnapshot {
                type_name: comp_type_name::<Name>().to_string(),
                ron: "(\"Updated Root\")".to_string(),
            }],
        }],
        ..prefab
    };

    {
        let mut ctx = game.ctx_mut();
        refresh_prefab_instance(&mut ctx, root_entity, &updated_prefab, None);
    }

    assert_eq!(
        game.ecs
            .get::<Name>(root_entity)
            .map(|name| name.0.as_str()),
        Some("Updated Root")
    );
    assert_eq!(
        game.ecs
            .get::<Velocity>(root_entity)
            .map(|velocity| (velocity.x, velocity.y)),
        Some((2.0, 4.0))
    );
}

#[test]
fn refresh_prefab_instance_applies_node_additions_and_removals_by_node_id() {
    let mut game = test_game();
    let prefab = PrefabAsset {
        id: PrefabId(1),
        name: "tree".to_string(),
        next_node_id: 3,
        root_node_id: 1,
        nodes: vec![
            PrefabNode {
                node_id: 1,
                parent_node_id: None,
                components: vec![ComponentSnapshot {
                    type_name: comp_type_name::<Name>().to_string(),
                    ron: "(\"Root\")".to_string(),
                }],
            },
            PrefabNode {
                node_id: 2,
                parent_node_id: Some(1),
                components: vec![ComponentSnapshot {
                    type_name: comp_type_name::<Name>().to_string(),
                    ron: "(\"Old Child\")".to_string(),
                }],
            },
        ],
    };

    let root_entity = {
        let mut ctx = game.ctx_mut();
        instantiate_prefab(&mut ctx, &prefab, Vec2::ZERO, None)
    };
    let old_child = find_entity_for_node(&game.ecs, root_entity, 2).unwrap();

    let updated_prefab = PrefabAsset {
        next_node_id: 4,
        nodes: vec![
            PrefabNode {
                node_id: 1,
                parent_node_id: None,
                components: vec![ComponentSnapshot {
                    type_name: comp_type_name::<Name>().to_string(),
                    ron: "(\"Root\")".to_string(),
                }],
            },
            PrefabNode {
                node_id: 3,
                parent_node_id: Some(1),
                components: vec![ComponentSnapshot {
                    type_name: comp_type_name::<Name>().to_string(),
                    ron: "(\"New Child\")".to_string(),
                }],
            },
        ],
        ..prefab
    };

    {
        let mut ctx = game.ctx_mut();
        refresh_prefab_instance(&mut ctx, root_entity, &updated_prefab, None);
    }

    let new_child = find_entity_for_node(&game.ecs, root_entity, 3).unwrap();
    assert!(game.ecs.get::<Name>(old_child).is_none());
    assert_eq!(get_parent(&game.ecs, new_child), Some(root_entity));
    assert_eq!(
        game.ecs.get::<Name>(new_child).map(|name| name.0.as_str()),
        Some("New Child")
    );
}

#[test]
fn refresh_prefab_instance_removes_deleted_prefab_components_without_local_override() {
    let mut game = test_game();
    let prefab = PrefabAsset {
        id: PrefabId(1),
        name: "mover".to_string(),
        next_node_id: 2,
        root_node_id: 1,
        nodes: vec![PrefabNode {
            node_id: 1,
            parent_node_id: None,
            components: vec![
                ComponentSnapshot {
                    type_name: comp_type_name::<Name>().to_string(),
                    ron: "(\"Mover\")".to_string(),
                },
                ComponentSnapshot {
                    type_name: comp_type_name::<Velocity>().to_string(),
                    ron: "(x:1.0,y:2.0)".to_string(),
                },
            ],
        }],
    };

    let root_entity = {
        let mut ctx = game.ctx_mut();
        instantiate_prefab(&mut ctx, &prefab, Vec2::ZERO, None)
    };
    assert!(game.ecs.has::<Velocity>(root_entity));

    let updated_prefab = PrefabAsset {
        nodes: vec![PrefabNode {
            node_id: 1,
            parent_node_id: None,
            components: vec![ComponentSnapshot {
                type_name: comp_type_name::<Name>().to_string(),
                ron: "(\"Mover\")".to_string(),
            }],
        }],
        ..prefab
    };

    {
        let mut ctx = game.ctx_mut();
        refresh_prefab_instance(&mut ctx, root_entity, &updated_prefab, None);
    }

    assert!(!game.ecs.has::<Velocity>(root_entity));
}

#[test]
fn refresh_prefab_instance_removes_parent_when_node_becomes_root_level() {
    let mut game = test_game();
    let prefab = PrefabAsset {
        id: PrefabId(1),
        name: "tree".to_string(),
        next_node_id: 3,
        root_node_id: 1,
        nodes: vec![
            PrefabNode {
                node_id: 1,
                parent_node_id: None,
                components: vec![ComponentSnapshot {
                    type_name: comp_type_name::<Name>().to_string(),
                    ron: "(\"Root\")".to_string(),
                }],
            },
            PrefabNode {
                node_id: 2,
                parent_node_id: Some(1),
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
    let child_entity = find_entity_for_node(&game.ecs, root_entity, 2).unwrap();
    assert_eq!(get_parent(&game.ecs, child_entity), Some(root_entity));

    let updated_prefab = PrefabAsset {
        nodes: vec![PrefabNode {
            node_id: 1,
            parent_node_id: None,
            components: vec![ComponentSnapshot {
                type_name: comp_type_name::<Name>().to_string(),
                ron: "(\"Root\")".to_string(),
            }],
        }],
        ..prefab
    };

    {
        let mut ctx = game.ctx_mut();
        refresh_prefab_instance(&mut ctx, root_entity, &updated_prefab, None);
    }

    assert_eq!(get_parent(&game.ecs, child_entity), None);
}

#[test]
fn refresh_prefab_instance_updates_root_transform_fields_but_keeps_position() {
    let mut game = test_game();
    let prefab = PrefabAsset {
        id: PrefabId(1),
        name: "root_transform".to_string(),
        next_node_id: 2,
        root_node_id: 1,
        nodes: vec![PrefabNode {
            node_id: 1,
            parent_node_id: None,
            components: vec![ComponentSnapshot {
                type_name: comp_type_name::<Transform>().to_string(),
                ron: ron::to_string(&Transform::default()).unwrap(),
            }],
        }],
    };

    let root_entity = {
        let mut ctx = game.ctx_mut();
        instantiate_prefab(&mut ctx, &prefab, Vec2::new(12.0, 34.0), None)
    };

    let updated_prefab = PrefabAsset {
        nodes: vec![PrefabNode {
            node_id: 1,
            parent_node_id: None,
            components: vec![ComponentSnapshot {
                type_name: comp_type_name::<Transform>().to_string(),
                ron: ron::to_string(&Transform {
                    visible: false,
                    position: Vec2::ZERO,
                    pivot: Pivot::TopLeft,
                })
                .unwrap(),
            }],
        }],
        ..prefab
    };

    {
        let mut ctx = game.ctx_mut();
        refresh_prefab_instance(&mut ctx, root_entity, &updated_prefab, None);
    }

    let transform = game.ecs.get::<Transform>(root_entity).copied().unwrap();
    assert_eq!(transform.position, Vec2::new(12.0, 34.0));
    assert!(!transform.visible);
    assert_eq!(transform.pivot, Pivot::TopLeft);
}
