use engine_core::prelude::*;

/// Identifies whether a linked prefab reference came from a root or child node component.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrefabLinkSource {
    /// The entity is the linked prefab root.
    Root,
    /// The entity is a linked prefab child node.
    Node,
}

/// Read-only display data for a linked prefab instance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrefabLinkDisplay {
    /// The component source that identified the link.
    pub source: PrefabLinkSource,
    /// Stable prefab asset id.
    pub prefab_id: PrefabId,
    /// Human-readable label for UI display.
    pub label: String,
}

/// Returns read-only display data for a linked prefab instance entity.
pub fn linked_prefab_display(
    ecs: &Ecs,
    prefab_library: &PrefabLibrary,
    entity: Entity,
) -> Option<PrefabLinkDisplay> {
    let (source, prefab_id) = if let Some(root) = ecs.get::<PrefabInstanceRoot>(entity) {
        (PrefabLinkSource::Root, root.prefab_id)
    } else if let Some(node) = ecs.get::<PrefabInstanceNode>(entity) {
        (PrefabLinkSource::Node, node.prefab_id)
    } else {
        return None;
    };

    let prefab_label = prefab_library
        .prefabs
        .get(&prefab_id)
        .map(|prefab| prefab.name.clone())
        .unwrap_or_else(|| prefab_id.to_string());

    Some(PrefabLinkDisplay {
        source,
        prefab_id,
        label: format!("Prefab: {prefab_label}"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_prefab(prefab_id: PrefabId, name: String) -> PrefabAsset {
        engine_core::prelude::create_prefab(prefab_id, name)
    }

    #[test]
    fn linked_prefab_display_uses_root_metadata_for_roots_and_node_metadata_for_children() {
        let mut ecs = Ecs::default();
        let root = ecs
            .create_entity()
            .with(Transform::default())
            .with(Name("Root".to_string()))
            .finish();
        let child = ecs
            .create_entity()
            .with(Transform::default())
            .with(Name("Child".to_string()))
            .finish();
        set_parent(&mut ecs, child, root);

        let prefab_id = PrefabId(7);
        let prefab = create_prefab(prefab_id, "Crate".to_string());
        let mut prefab_library = PrefabLibrary::default();
        prefab_library.prefabs.insert(prefab_id, prefab);

        ecs.add_component_to_entity(
            root,
            PrefabInstanceRoot {
                prefab_id,
            },
        );
        ecs.add_component_to_entity(
            root,
            PrefabInstanceNode {
                prefab_id,
                node_id: 1,
                root_entity: root,
            },
        );
        ecs.add_component_to_entity(
            child,
            PrefabInstanceNode {
                prefab_id,
                node_id: 2,
                root_entity: root,
            },
        );

        let root_display = linked_prefab_display(&ecs, &prefab_library, root).unwrap();
        let child_display = linked_prefab_display(&ecs, &prefab_library, child).unwrap();

        assert_eq!(root_display.source, PrefabLinkSource::Root);
        assert_eq!(root_display.label, "Prefab: Crate");
        assert_eq!(child_display.source, PrefabLinkSource::Node);
        assert_eq!(child_display.label, "Prefab: Crate");
    }

    #[test]
    fn linked_prefab_display_falls_back_to_prefab_id_when_asset_is_missing() {
        let mut ecs = Ecs::default();
        let entity = ecs
            .create_entity()
            .with(Transform::default())
            .with(Name("Entity".to_string()))
            .finish();

        ecs.add_component_to_entity(
            entity,
            PrefabInstanceRoot {
                prefab_id: PrefabId(42),
            },
        );

        let prefab_library = PrefabLibrary::default();
        let display = linked_prefab_display(&ecs, &prefab_library, entity).unwrap();

        assert_eq!(display.source, PrefabLinkSource::Root);
        assert_eq!(display.label, "Prefab: 42");
    }
}
