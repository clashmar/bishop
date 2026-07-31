use super::super::room::RoomId;
use super::super::room_layers::RoomLayer;
use super::super::world::WorldId;

/// One frame on the overlay stack, tracking which world, room, and layer to return to.
#[derive(Debug, Clone, PartialEq)]
pub struct OverlayFrame {
    /// The world to return to.
    pub world: WorldId,
    /// The room the player was in when this overlay was entered.
    pub room: RoomId,
    /// The authored room layer active when this overlay was entered.
    pub layer: RoomLayer,
}
