use crate::ecs::component::comp_type_name;
use crate::ecs::components::prefab_instance::PrefabInstanceNode;
use crate::ecs::entity::{Children, Parent};
use crate::game::GameCtxMut;
use serde::{Deserialize, Serialize};
use std::any::Any;
use std::collections::HashMap;

/// A single serialized component.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ComponentSnapshot {
    pub type_name: String,
    pub ron: String,
}

/// A collection of components belonging to a specific entity.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EntitySnapshot {
    pub entity: Entity,
    pub components: Vec<ComponentSnapshot>,
}

/// A complete hierarchy or group of captured entities.
pub type GroupSnapshot = Vec<EntitySnapshot>;

/// Capture the whole entity hierarchy that starts at `root`.
pub fn capture_subtree(ecs: &mut Ecs, root: Entity) -> GroupSnapshot {
    let mut result = Vec::new();
    let mut stack = vec![root];

    while let Some(e) = stack.pop() {
        // Capture the components of the current entity
        let components = capture_entity(ecs, e);
        result.push(EntitySnapshot {
            entity: e,
            components,
        });

        // Push its children (if any) on the stack
        if let Some(children) = ecs.get::<Children>(e) {
            for &c in &children.entities {
                stack.push(c);
            }
        }
    }
    result
}

/// Restore a previously captured subtree.  
pub fn restore_subtree(ctx: &mut GameCtxMut<'_>, saved: &GroupSnapshot) {
    for snapshot in saved {
        restore_entity(ctx, snapshot.entity, snapshot.components.clone());
    }
}

/// Restores an entity into the ECS from its captured component snapshots.
pub fn restore_entity(
    ctx: &mut GameCtxMut<'_>,
    entity: Entity,
    components: Vec<ComponentSnapshot>,
) {
    for comp in components {
        let component_reg = inventory::iter::<ComponentRegistry>()
            .find(|r| r.type_name == comp.type_name)
            .expect("Component not registered");

        let mut boxed = (component_reg.from_ron_component)(comp.ron);
        (component_reg.post_create)(&mut *boxed, &entity, ctx);
        (component_reg.inserter)(ctx.ecs(), entity, boxed);

        ctx.ecs().run_registered_on_insert(component_reg, entity);
    }
}

/// Deserialize a component snapshot and remap entity ID references.
/// Returns the registry entry and the remapped component, ready for insertion
/// via `Ecs::insert_component_dyn`.
pub fn restore_component_with_remap(
    comp: &ComponentSnapshot,
    id_map: &HashMap<Entity, Entity>,
) -> Option<(&'static ComponentRegistry, Box<dyn Any>)> {
    let reg = inventory::iter::<ComponentRegistry>()
        .find(|r| r.type_name == comp.type_name)?;

    let mut boxed = (reg.from_ron_component)(comp.ron.clone());

    if comp.type_name == comp_type_name::<Parent>() {
        if let Some(parent) = boxed.as_mut().downcast_mut::<Parent>()
            && let Some(&new_parent) = id_map.get(&parent.0)
        {
            parent.0 = new_parent;
        }
    } else if comp.type_name == comp_type_name::<Children>() {
        if let Some(children) = boxed.as_mut().downcast_mut::<Children>() {
            for child in &mut children.entities {
                if let Some(&new_child) = id_map.get(child) {
                    *child = new_child;
                }
            }
        }
    } else if comp.type_name == comp_type_name::<PrefabInstanceNode>()
        && let Some(node) = boxed.as_mut().downcast_mut::<PrefabInstanceNode>()
        && let Some(&new_root) = id_map.get(&node.root_entity)
    {
        node.root_entity = new_root;
    }

    Some((reg, boxed))
}

/// Generates a `capture_entity` implementation that works for any component
/// registered with `ecs_component!`.
#[macro_export]
macro_rules! impl_capture_entity {
    () => {
        use $crate::ecs::component_registry::ComponentRegistry;
        use $crate::ecs::ecs::Ecs;
        use $crate::ecs::entity::Entity;

        /// Walks the component registry and extracts every component the entity owns.
        pub fn capture_entity(
            ecs: &mut Ecs,
            entity: Entity,
        ) -> Vec<$crate::ecs::capture::ComponentSnapshot> {
            let mut components = Vec::new();

            for reg in inventory::iter::<ComponentRegistry> {
                if (reg.has)(ecs, entity) {
                    let boxed = (reg.clone)(ecs, entity);
                    let ron = (reg.to_ron_component)(&*boxed);

                    components.push($crate::ecs::capture::ComponentSnapshot {
                        type_name: reg.type_name.to_string(),
                        ron,
                    });
                }
            }
            components
        }
    };
}

impl_capture_entity!();
