use crate::constants::lighting;
use crate::ecs::Entity;
use crate::game::Game;
use crate::worlds::room::{Room, RoomVariant};
use bishop::prelude::Color;
use serde::{Deserialize, Serialize};
use serde_with::{FromInto, serde_as};

/// Heavy room-local payload data hydrated on demand.
#[serde_as]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RoomPayload {
    /// Room tilemap variants.
    pub variants: Vec<RoomVariant>,
    /// Room-local entity snapshots.
    pub entities: Vec<Entity>,
    /// Room darkness level.
    pub darkness: f32,
    /// Room ambient color.
    #[serde_as(as = "FromInto<[f32; 4]>")]
    pub ambient_color: Color,
    /// Room exposure multiplier.
    pub exposure: f32,
}

impl Default for RoomPayload {
    fn default() -> Self {
        Self {
            variants: vec![],
            entities: vec![],
            darkness: lighting::DEFAULT_ROOM_DARKNESS,
            ambient_color: lighting::DEFAULT_ROOM_AMBIENT_COLOR,
            exposure: lighting::DEFAULT_ROOM_EXPOSURE,
        }
    }
}

impl RoomPayload {
    pub fn capture(game: &Game, room: &Room) -> Self {
        let mut entities: Vec<_> = game.ecs.entities_in_room(room.id).iter().copied().collect();
        entities.sort_unstable();

        Self {
            variants: room.variants.clone(),
            entities,
            darkness: room.darkness,
            ambient_color: room.ambient_color,
            exposure: room.exposure,
        }
    }

    pub fn apply(self, room: &mut Room) {
        room.variants = self.variants;
        room.darkness = self.darkness;
        room.ambient_color = self.ambient_color;
        room.exposure = self.exposure;
    }

    pub fn clear(room: &mut Room) {
        room.variants.clear();
        room.darkness = lighting::DEFAULT_ROOM_DARKNESS;
        room.ambient_color = lighting::DEFAULT_ROOM_AMBIENT_COLOR;
        room.exposure = lighting::DEFAULT_ROOM_EXPOSURE;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::worlds::RoomId;

    #[test]
    fn default_payload_uses_neutral_ambient_settings() {
        let payload = RoomPayload::default();

        assert_eq!(payload.darkness, lighting::DEFAULT_ROOM_DARKNESS);
        assert_eq!(payload.ambient_color, lighting::DEFAULT_ROOM_AMBIENT_COLOR);
        assert_eq!(payload.exposure, lighting::DEFAULT_ROOM_EXPOSURE);
    }

    fn room_with_ambient_settings(game: &mut Game, room_id: RoomId) -> Room {
        let mut room = Room::new(&mut game.ecs, room_id, 16.0);
        room.darkness = 0.7;
        room.ambient_color = Color::new(0.3, 0.4, 0.5, 1.0);
        room.exposure = 1.8;
        room
    }

    #[test]
    fn capture_and_apply_preserve_ambient_settings() {
        let mut game = Game::default();
        let room = room_with_ambient_settings(&mut game, RoomId(1));

        let payload = RoomPayload::capture(&game, &room);
        let mut restored = Room::new(&mut game.ecs, RoomId(2), 16.0);
        payload.apply(&mut restored);

        assert_eq!(restored.darkness, room.darkness);
        assert_eq!(restored.ambient_color, room.ambient_color);
        assert_eq!(restored.exposure, room.exposure);
    }

    #[test]
    fn clear_resets_ambient_settings() {
        let mut game = Game::default();
        let mut room = room_with_ambient_settings(&mut game, RoomId(1));

        RoomPayload::clear(&mut room);

        assert_eq!(room.darkness, lighting::DEFAULT_ROOM_DARKNESS);
        assert_eq!(room.ambient_color, lighting::DEFAULT_ROOM_AMBIENT_COLOR);
        assert_eq!(room.exposure, lighting::DEFAULT_ROOM_EXPOSURE);
    }

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
