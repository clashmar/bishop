use crate::assets::AssetKey;
use crate::worlds::{RoomId, WorldId};

/// Payload residency keys tracked alongside asset residency.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum PayloadKey {
    Global,
    World(WorldId),
    Room(RoomId),
}

/// Any residency-tracked key managed by the hydration coordinator.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ResidencyKey {
    Asset(AssetKey),
    Payload(PayloadKey),
}
