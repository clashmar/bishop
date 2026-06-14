use super::super::world::WorldId;
use super::super::room::RoomId;

/// One frame on the overlay stack, tracking which world and room to return to.
#[derive(Debug, Clone, PartialEq)]
pub struct OverlayFrame {
    /// The world to return to.
    pub world: WorldId,
    /// The room the player was in when this overlay was entered.
    pub room: RoomId,
}
