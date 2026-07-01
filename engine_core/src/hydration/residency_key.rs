use crate::assets::AssetKey;
use crate::worlds::{RoomId, WorldId};

/// Scope-level residency keys tracked alongside asset residency.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ScopeKey {
    Global,
    World(WorldId),
    Room(RoomId),
}

/// Any residency-tracked key managed by the hydration coordinator.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ResidencyKey {
    Asset(AssetKey),
    Scope(ScopeKey),
}
