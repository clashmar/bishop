use crate::assets::AssetKey;
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
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ResourceClass {
    Texture,
    Script,
    Audio,
    Prefab,
}

impl ResourceClass {
    /// Classifies a concrete asset key into its hydration resource class.
    pub fn for_asset_key(asset: AssetKey) -> Option<Self> {
        match asset {
            AssetKey::Sprite(_) => Some(Self::Texture),
            AssetKey::Script(_) => Some(Self::Script),
            AssetKey::Sound(_) => Some(Self::Audio),
            AssetKey::Prefab(_) => Some(Self::Prefab),
            AssetKey::Toml(_) => None,
        }
    }

    /// Returns the short overlay label for this resource class.
    pub fn display_abbreviation(self) -> &'static str {
        match self {
            Self::Texture => "T",
            Self::Script => "S",
            Self::Audio => "A",
            Self::Prefab => "P",
        }
    }
}

/// Summary of what a scope claims. Per-scope per-class owned assets.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResourceClaim {
    pub scope: HydrationScope,
    pub class: ResourceClass,
    pub assets: Vec<AssetKey>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecs::{ScriptId, SoundId, SpriteId, TomlId};
    use crate::prefab::PrefabId;

    #[test]
    fn hydration_scope_variants_are_distinct() {
        let boot = HydrationScope::Boot;
        let global = HydrationScope::Global;
        let world_a = HydrationScope::World(WorldId(1));
        let world_b = HydrationScope::World(WorldId(2));
        let room_a = HydrationScope::Room(RoomId(3));
        let entity_a = HydrationScope::Entity(Entity(4));

        assert_eq!(HydrationScope::Boot, HydrationScope::Boot);
        assert_eq!(HydrationScope::Global, HydrationScope::Global);
        assert_eq!(world_a, HydrationScope::World(WorldId(1)));

        assert_ne!(world_a, world_b);

        assert_ne!(boot, global);
        assert_ne!(boot, world_a);
        assert_ne!(global, room_a);
        assert_ne!(world_a, room_a);
        assert_ne!(room_a, entity_a);
    }

    #[test]
    fn resource_class_maps_hydratable_asset_keys() {
        assert_eq!(
            ResourceClass::for_asset_key(AssetKey::Sprite(SpriteId(1))),
            Some(ResourceClass::Texture)
        );
        assert_eq!(
            ResourceClass::for_asset_key(AssetKey::Script(ScriptId(2))),
            Some(ResourceClass::Script)
        );
        assert_eq!(
            ResourceClass::for_asset_key(AssetKey::Sound(SoundId(3))),
            Some(ResourceClass::Audio)
        );
        assert_eq!(
            ResourceClass::for_asset_key(AssetKey::Prefab(PrefabId(4))),
            Some(ResourceClass::Prefab)
        );
        assert_eq!(
            ResourceClass::for_asset_key(AssetKey::Toml(TomlId(5))),
            None
        );
    }

    #[test]
    fn resource_claim_stores_scope_class_and_assets() {
        let claim = ResourceClaim {
            scope: HydrationScope::Room(RoomId(7)),
            class: ResourceClass::Texture,
            assets: vec![AssetKey::Sprite(SpriteId(12))],
        };

        assert_eq!(claim.scope, HydrationScope::Room(RoomId(7)));
        assert_eq!(claim.class, ResourceClass::Texture);
        assert_eq!(claim.assets, vec![AssetKey::Sprite(SpriteId(12))]);
    }
}
