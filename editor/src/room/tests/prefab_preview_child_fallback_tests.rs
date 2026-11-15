use super::*;

fn component_snapshot(type_name: String, ron: String) -> ComponentSnapshot {
    ComponentSnapshot { type_name, ron }
}

fn camera_node(
    node_id: usize,
    parent_node_id: Option<usize>,
    position: Vec2,
) -> Result<PrefabNode, ron::Error> {
    let transform = Transform {
        position,
        ..Default::default()
    };
    let transform_ron = ron::to_string(&transform)?;
    let camera_ron = ron::to_string(&RoomCamera::default())?;

    Ok(PrefabNode {
        node_id,
        parent_node_id,
        components: vec![
            component_snapshot(comp_type_name::<Transform>().to_string(), transform_ron),
            component_snapshot(comp_type_name::<RoomCamera>().to_string(), camera_ron),
        ],
    })
}

#[test]
fn collect_fallback_visuals_excludes_child_icons_but_keeps_root_icon() -> Result<(), ron::Error> {
    let root_position = vec2(10.0, 20.0);
    let prefab = PrefabAsset {
        id: PrefabId(1),
        name: "test".into(),
        next_node_id: 3,
        root_node_id: 1,
        nodes: vec![
            camera_node(1, None, root_position)?,
            camera_node(2, Some(1), vec2(30.0, 40.0))?,
        ],
    };

    let visuals = collect_fallback_visuals(&prefab);

    assert_eq!(visuals.len(), 1);
    assert_eq!(visuals[0].0, root_position);
    assert!(matches!(visuals[0].2, EntityVisual::CameraIcon));
    Ok(())
}
