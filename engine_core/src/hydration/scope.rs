use crate::ecs::Entity;
use crate::worlds::{RoomId, WorldId};

/// Which scope owns hydrated resources.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum HydrationScope {
    /// Startup/bootstrap resources.
    Boot,
    /// Session-long resources.
    Global,
    /// Resources needed anywhere within one world.
    World(WorldId),
    /// Resources needed within one room.
    Room(RoomId),
    /// Resources owned by a specific entity or instance.
    Entity(Entity),
}

/// What kind of hydrated resource is being tracked.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ResourceClass {
    Texture,
    Script,
    Audio,
    Prefab,
}

/// Summary of what a scope claims. Per-scope per-class counts only.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResourceClaim {
    pub scope: HydrationScope,
    pub class: ResourceClass,
    pub count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hydration_scope_variants_are_distinct() {
        let boot = HydrationScope::Boot;
        let global = HydrationScope::Global;
        let world_a = HydrationScope::World(WorldId(1));
        let world_b = HydrationScope::World(WorldId(2));
        let room_a = HydrationScope::Room(RoomId(3));
        let entity_a = HydrationScope::Entity(Entity(4));

        // Same variant, same data → equal
        assert_eq!(HydrationScope::Boot, HydrationScope::Boot);
        assert_eq!(HydrationScope::Global, HydrationScope::Global);
        assert_eq!(world_a, HydrationScope::World(WorldId(1)));

        // Same variant, different data → not equal
        assert_ne!(world_a, world_b);

        // Different variants → not equal
        assert_ne!(boot, global);
        assert_ne!(boot, world_a);
        assert_ne!(global, room_a);
        assert_ne!(world_a, room_a);
        assert_ne!(room_a, entity_a);
    }

    #[test]
    fn resource_claim_stores_scope_class_and_count() {
        let claim = ResourceClaim {
            scope: HydrationScope::Room(RoomId(7)),
            class: ResourceClass::Texture,
            count: 12,
        };

        assert_eq!(claim.scope, HydrationScope::Room(RoomId(7)));
        assert_eq!(claim.class, ResourceClass::Texture);
        assert_eq!(claim.count, 12);
    }
}
