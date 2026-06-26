use crate::ecs::Entity;
use crate::game::Game;
use crate::worlds::room::{Room, RoomVariant};
use serde::{Deserialize, Serialize};

/// Heavy room-local payload data hydrated on demand.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct RoomPayload {
    /// Room tilemap variants.
    pub variants: Vec<RoomVariant>,
    /// Room-local entity snapshots.
    pub entities: Vec<Entity>,
    /// Room darkness level.
    pub darkness: f32,
}

impl RoomPayload {
    pub fn capture(game: &Game, room: &Room) -> Self {
        let mut entities: Vec<_> = game.ecs.entities_in_room(room.id).iter().copied().collect();
        entities.sort_unstable();

        Self {
            variants: room.variants.clone(),
            entities,
            darkness: room.darkness,
        }
    }

    pub fn apply(self, room: &mut Room) {
        room.variants = self.variants;
        room.darkness = self.darkness;
    }

    pub fn clear(room: &mut Room) {
        room.variants.clear();
        room.darkness = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::worlds::RoomId;

    #[test]
    fn capture_sorts_room_entities_by_entity_id() {
        let mut game = Game::default();
        let room = Room::new(&mut game.ecs, RoomId(1), 16.0);

        let entity_1 = game.ecs.create_entity().with_current_room(room.id).finish();
        let entity_2 = game.ecs.create_entity().with_current_room(room.id).finish();
        let entity_3 = game.ecs.create_entity().with_current_room(room.id).finish();

        let payload = RoomPayload::capture(&game, &room);
        let mut expected: Vec<_> = game.ecs.entities_in_room(room.id).iter().copied().collect();
        expected.sort_unstable();

        assert!(expected.contains(&entity_1));
        assert!(expected.contains(&entity_2));
        assert!(expected.contains(&entity_3));
        assert_eq!(payload.entities, expected);
    }
}
