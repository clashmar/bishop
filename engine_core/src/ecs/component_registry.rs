// engine_core/src/ecs/component_registry.rs
use crate::ecs::component::Component;
use crate::ecs::{ecs::Ecs, entity::Entity};
use crate::game::EngineCtxMut;
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
    pub post_create: fn(&mut dyn Any, &Entity, &mut dyn EngineCtxMut),
    /// Called optionally when a component is removed from an entity.
    pub post_remove: fn(&mut dyn Any, &Entity, &mut dyn EngineCtxMut),
    /// Converts the rust component to a lua type.
    pub to_lua: fn(&Lua, &dyn Any) -> mlua::Result<Value>,
    /// Converts the lua value back to the rust component.
    pub from_lua: fn(&Lua, Value) -> mlua::Result<Box<dyn Any>>,
    /// Returns the Lua schema for this component (field names and types).
    pub lua_schema: fn() -> &'static [(&'static str, &'static str)],
}

/// Factory that works for any component that implements `Component + Default`.
pub fn generic_factory<T>(ecs: &mut Ecs, entity: Entity)
where
    T: Component + Default + 'static,
{
    // Directly insert the default component into its typed store.
    ecs.get_store_mut::<T>().insert(entity, T::default());
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
pub fn noop_post_create(_any: &mut dyn Any, _entity: &Entity, _ctx: &mut dyn EngineCtxMut) {}

/// Default implementation used when a component does not need any post-remove work.
pub fn noop_post_remove(_any: &mut dyn Any, _entity: &Entity, _ctx: &mut dyn EngineCtxMut) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn components_are_sorted_by_type_name() {
        assert!(
            COMPONENTS
                .windows(2)
                .all(|pair| pair[0].type_name <= pair[1].type_name)
        );
    }
}
