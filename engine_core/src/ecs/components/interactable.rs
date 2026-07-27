use crate::ecs::ecs::Ecs;
use crate::ecs::entity::Entity;
use crate::ecs::{CurrentRoom, Transform};
use crate::inspector_module;
use ecs_component::ecs_component;
use reflect_derive::Reflect;
use serde::{Deserialize, Serialize};

/// Component for interactable entities.
#[ecs_component]
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct Interactable {
    /// Maximum interaction distance.
    pub range: f32,
    // TODO: Add priority,
    // enabled/disabled,
    // prompt,
    // facing
    // event dispatch
}
inspector_module!(Interactable);

impl Default for Interactable {
    fn default() -> Self {
        Self { range: 20.0 }
    }
}

/// Returns the best interactable entity candidate for the player in the current room/layer.
pub fn find_best_interactable(ecs: &Ecs) -> Option<Entity> {
    let player = ecs.get_player_entity()?;
    let player_pos = ecs.get_player_transform()?.position;
    let player_room = ecs.get::<CurrentRoom>(player).copied()?;

    find_best_interactable_in_layer(ecs, player_room.room_id, player_room.layer, player_pos)
}

pub fn find_best_interactable_in_layer(
    ecs: &Ecs,
    room_id: crate::worlds::RoomId,
    layer: crate::worlds::RoomLayer,
    source_pos: bishop::prelude::Vec2,
) -> Option<Entity> {
    let interactables = ecs.get_store::<Interactable>();
    let positions = ecs.get_store::<Transform>();

    let mut best: Option<(Entity, f32)> = None;

    for &entity in ecs.entities_in_room_layer(room_id, layer) {
        ecs.assert_room_membership(room_id, entity);

        let Some(interactable) = interactables.get(entity) else {
            continue;
        };

        let Some(pos) = positions.get(entity).map(|transform| transform.position) else {
            continue;
        };

        let dist = source_pos.distance(pos);
        if dist > interactable.range {
            continue;
        }

        match best {
            None => best = Some((entity, dist)),
            Some((_, best_dist)) if dist < best_dist => best = Some((entity, dist)),
            _ => {}
        }
    }

    best.map(|(entity, _)| entity)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::worlds::{RoomId, RoomLayer};
    use bishop::prelude::Vec2;

    #[test]
    fn find_best_interactable_ignores_other_layers_in_the_same_room() {
        let mut ecs = Ecs::default();
        let room_id = RoomId(3);

        let front_entity = ecs.create_entity()
            .with(Transform {
                position: Vec2::new(8.0, 0.0),
                ..Default::default()
            })
            .with(Interactable { range: 100.0 })
            .with_current_room(room_id)
            .finish();

        let player = ecs.create_entity()
            .with(Transform {
                position: Vec2::new(0.0, 0.0),
                ..Default::default()
            })
            .with(crate::ecs::Player::default())
            .with_current_room(room_id)
            .finish();

        ecs.create_entity()
            .with(Transform {
                position: Vec2::new(1.0, 0.0),
                ..Default::default()
            })
            .with(Interactable { range: 100.0 })
            .with_current_room_layer(room_id, RoomLayer::Back)
            .finish();

        assert_eq!(ecs.get_player_entity(), Some(player));
        assert_eq!(find_best_interactable(&ecs), Some(front_entity));
    }

    #[test]
    fn find_best_interactable_ignores_other_rooms() {
        let mut ecs = Ecs::default();
        let player_room = RoomId(3);
        let other_room = RoomId(4);

        let player = ecs.create_entity()
            .with(Transform {
                position: Vec2::new(0.0, 0.0),
                ..Default::default()
            })
            .with(crate::ecs::Player::default())
            .with_current_room(player_room)
            .finish();

        ecs.create_entity()
            .with(Transform {
                position: Vec2::new(1.0, 0.0),
                ..Default::default()
            })
            .with(Interactable { range: 100.0 })
            .with_current_room(other_room)
            .finish();

        assert_eq!(ecs.get_player_entity(), Some(player));
        assert_eq!(find_best_interactable(&ecs), None);
    }
}
