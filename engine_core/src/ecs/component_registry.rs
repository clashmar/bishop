use crate::ecs::component::Component;
use crate::ecs::ComponentStore;
use crate::ecs::{ecs::Ecs, entity::Entity};
use crate::game::GameCtxMut;
use mlua::Lua;
use mlua::Value;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::any::{Any, TypeId};

/// Human‑readable names of all components that have been registered with `ecs_component!`.
pub static COMPONENTS: Lazy<Vec<&'static ComponentRegistry>> = Lazy::new(|| {
    let mut components: Vec<_> = inventory::iter::<ComponentRegistry>.into_iter().collect();
    components.sort_by(|left, right| left.type_name.cmp(right.type_name));
    components
});

inventory::collect!(ComponentRegistry);

/// Trait for generating Lua schema information
pub trait LuaSchema {
    fn lua_schema() -> &'static [(&'static str, &'static str)];
}

/// One entry for a concrete component type.
pub struct ComponentRegistry {
    /// Human‑readable identifier that will appear in the save file.
    pub type_name: &'static str,
    /// The concrete `ComponentStore<T>`’s `TypeId`.
    pub type_id: TypeId,
    /// Convert a concrete `ComponentStore<T>` (as a reference) into a `String`.
    pub to_ron: fn(&dyn Any) -> String,
    /// Convert a `String` back into a boxed concrete store.
    pub from_ron: fn(String) -> Box<dyn Any + Send + Sync>,
    /// Factory that creates the component (and its dependencies) for an entity.
    pub factory: fn(&mut Ecs, Entity),
    /// Ensures the component exists without overwriting an existing value.
    pub ensure: fn(&mut Ecs, Entity),
    /// Returns true if the supplied entity already owns this component.
    pub has: fn(&Ecs, Entity) -> bool,
    // Removes the component for `entity` from the concrete store.
    pub remove: fn(&mut Ecs, Entity),
    /// Function that knows how to write a boxed component back into the world.
    pub inserter: fn(&mut Ecs, Entity, Box<dyn Any>),
    /// Clones the concrete component for `entity` and returns it boxed as `dyn Any`.
    pub clone: fn(&Ecs, Entity) -> Box<dyn Any>,
    /// Serialize a single component.
    pub to_ron_component: fn(&dyn Any) -> String,
    /// Deserialize a single component.
    pub from_ron_component: fn(String) -> Box<dyn Any>,
    /// Called for optional run post‑create logic. If `None` the engine will do nothing.
    pub post_create: for<'a> fn(&mut dyn Any, &Entity, &mut GameCtxMut<'a>),
    /// Called optionally when a component is removed from an entity.
    pub post_remove: for<'a> fn(&mut dyn Any, &Entity, &mut GameCtxMut<'a>),
    /// Converts the rust component to a lua type.
    pub to_lua: fn(&Lua, &dyn Any) -> mlua::Result<Value>,
    /// Converts the lua value back to the rust component.
    pub from_lua: fn(&Lua, Value) -> mlua::Result<Box<dyn Any>>,
    /// Returns the Lua schema for this component (field names and types).
    pub lua_schema: fn() -> &'static [(&'static str, &'static str)],
    /// Whether this component should be visible through the public Lua API.
    pub is_public_lua_api: bool,
    /// Returns the highest entity id present in the store, or `None` if empty.
    pub max_entity_id: fn(&dyn Any) -> Option<usize>,
    /// Called on every component insertion.
    pub on_insert: for<'a> fn(&mut dyn Any, &Entity, &mut Ecs),
    /// Called on every component removal.
    pub on_remove: for<'a> fn(&mut dyn Any, &Entity, &mut Ecs),
    /// When true, this component's mutation should go through replace_component
    /// rather than raw get_mut field edits.
    pub guarded: bool,
}

/// Factory that works for any component that implements `Component + Default`.
pub fn generic_factory<T>(ecs: &mut Ecs, entity: Entity)
where
    T: Component + Default + 'static,
{
    // Directly insert the default component into its typed store.
    ecs.get_store_mut::<T>().insert(entity, T::default());
}

pub fn generic_ensure<T>(ecs: &mut Ecs, entity: Entity)
where
    T: Component + Default + 'static,
{
    let store = ecs.get_store_mut::<T>();
    if !store.contains(entity) {
        store.insert(entity, T::default());
    }
}

pub fn has_component<T>(world: &Ecs, entity: Entity) -> bool
where
    T: Component + 'static,
{
    world.get_store::<T>().contains(entity)
}

/// Helper that erases an entity from a concrete `ComponentStore<T>`.
pub fn erase_from_store<T>(ecs: &mut Ecs, entity: Entity)
where
    T: Component + 'static,
{
    ecs.get_store_mut::<T>().remove(entity);
}

/// Inserts a concrete component that has been boxed as `dyn Any`.
pub fn generic_inserter<T>(ecs: &mut Ecs, entity: Entity, boxed: Box<dyn Any>)
where
    T: Component + 'static,
{
    let type_id = TypeId::of::<ComponentStore<T>>();
    if let Some(reg) = COMPONENTS.iter().find(|reg| reg.type_id == type_id) {
        (reg.ensure)(ecs, entity);
    }

    let concrete = *boxed
        .downcast::<T>()
        .expect("ComponentEntry contains wrong type");
    ecs.get_store_mut::<T>().insert(entity, concrete);
}

#[derive(Serialize, Deserialize, Clone)]
pub struct StoredComponent {
    pub type_name: String,
    pub data: String,
}

/// Default implementation used when a component does not need any post-create work.
pub fn noop_post_create(_any: &mut dyn Any, _entity: &Entity, _ctx: &mut GameCtxMut<'_>) {}

/// Default implementation used when a component does not need any post-remove work.
pub fn noop_post_remove(_any: &mut dyn Any, _entity: &Entity, _ctx: &mut GameCtxMut<'_>) {}

/// No-op on_insert hook (default for components without custom lifecycle logic).
pub fn noop_on_insert(_component: &mut dyn Any, _entity: &Entity, _ecs: &mut Ecs) {}

/// No-op on_remove hook (default for components without custom lifecycle logic).
pub fn noop_on_remove(_component: &mut dyn Any, _entity: &Entity, _ecs: &mut Ecs) {}

/// Returns the components exposed through the public Lua API.
pub fn public_lua_components() -> impl Iterator<Item = &'static ComponentRegistry> {
    COMPONENTS
        .iter()
        .copied()
        .filter(|reg| reg.is_public_lua_api)
}

/// Finds a component by name only if it is exposed through the public Lua API.
pub fn find_public_lua_component(type_name: &str) -> Option<&'static ComponentRegistry> {
    public_lua_components().find(|reg| reg.type_name == type_name)
}

/// Finds a component by name for public Lua use.
pub fn public_lua_component(type_name: &str) -> Result<&'static ComponentRegistry, String> {
    if let Some(reg) = find_public_lua_component(type_name) {
        Ok(reg)
    } else if COMPONENTS.iter().any(|reg| reg.type_name == type_name) {
        Err(format!("Component '{type_name}' is not available to Lua"))
    } else {
        Err(format!("Unknown component '{type_name}'"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecs::component::comp_type_name;
    use crate::ecs::components::prefab_instance::{
        PrefabInstanceNode, PrefabInstanceRoot, PrefabOverrides,
    };
    use crate::ecs::MotionBody;
    use crate::ecs::{Grounded, PhysicsBody};

    #[test]
    fn registry_has_on_insert_field() {
        // Construct a minimal registry entry — will fail because fields don't exist yet
        let entry = ComponentRegistry {
            type_name: "Test",
            type_id: std::any::TypeId::of::<u32>(),
            to_ron: |_: &dyn Any| String::new(),
            from_ron: |_: String| Box::new(()) as Box<dyn Any + Send + Sync>,
            to_ron_component: |_: &dyn Any| String::new(),
            from_ron_component: |_: String| Box::new(()) as Box<dyn Any>,
            factory: |_: &mut Ecs, _: Entity| {},
            ensure: |_: &mut Ecs, _: Entity| {},
            has: |_: &Ecs, _: Entity| false,
            remove: |_: &mut Ecs, _: Entity| {},
            inserter: |_: &mut Ecs, _: Entity, _: Box<dyn Any>| {},
            clone: |_: &Ecs, _: Entity| Box::new(0u32) as Box<dyn Any>,
            post_create: |_: &mut dyn Any, _: &Entity, _: &mut GameCtxMut<'_>| {},
            post_remove: |_: &mut dyn Any, _: &Entity, _: &mut GameCtxMut<'_>| {},
            to_lua: |_: &Lua, _: &dyn Any| Ok(mlua::Value::Nil),
            from_lua: |_: &Lua, _: Value| Ok(Box::new(()) as Box<dyn Any>),
            lua_schema: || &[],
            is_public_lua_api: true,
            max_entity_id: |_: &dyn Any| None,
            on_insert: noop_on_insert,
            on_remove: noop_on_remove,
            guarded: false,
        };
        // Verify noops don't panic
        let mut val: Box<dyn Any> = Box::new(42u32);
        (entry.on_insert)(&mut *val, &Entity(1), &mut Ecs::default());
        (entry.on_remove)(&mut *val, &Entity(1), &mut Ecs::default());
    }

    #[test]
    fn components_are_sorted_by_type_name() {
        assert!(COMPONENTS
            .windows(2)
            .all(|pair| pair[0].type_name <= pair[1].type_name));
    }

    #[test]
    fn prefab_metadata_components_are_not_public_lua_api() {
        let hidden = [
            comp_type_name::<PrefabInstanceNode>(),
            comp_type_name::<PrefabInstanceRoot>(),
            comp_type_name::<PrefabOverrides>(),
        ];

        for type_name in hidden {
            let reg = COMPONENTS
                .iter()
                .find(|reg| reg.type_name == type_name)
                .unwrap_or_else(|| panic!("missing registry entry for {type_name}"));

            assert!(!reg.is_public_lua_api, "{type_name} should be private");
        }
    }

    #[test]
    fn motion_body_is_not_public_lua_api() {
        let type_name = comp_type_name::<MotionBody>();
        let reg = COMPONENTS
            .iter()
            .find(|reg| reg.type_name == type_name)
            .unwrap_or_else(|| panic!("missing registry entry for {type_name}"));

        assert!(!reg.is_public_lua_api, "{type_name} should be private");
    }

    #[test]
    fn inserter_preserves_existing_dependency_state() {
        let mut ecs = Ecs::default();
        let entity = Entity(1);
        ecs.get_store_mut::<Grounded>()
            .insert(entity, Grounded(true));

        generic_inserter::<PhysicsBody>(&mut ecs, entity, Box::new(PhysicsBody));

        assert_eq!(
            ecs.get::<Grounded>(entity).map(|grounded| grounded.0),
            Some(true)
        );
    }
}
