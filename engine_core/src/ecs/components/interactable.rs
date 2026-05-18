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

/// Returns the best interactable entity candidate for the player in the current room.
pub fn find_best_interactable(ecs: &Ecs) -> Option<Entity> {
    let player = ecs.get_player_entity()?;
    let player_pos = ecs.get_player_transform()?.position;

    let player_room = ecs.get::<CurrentRoom>(player).map(|r| r.0)?;

    let interactables = ecs.get_store::<Interactable>();
    let positions = ecs.get_store::<Transform>();

    let mut best: Option<(Entity, f32)> = None;

    for &entity in ecs.entities_in_room(player_room) {
        ecs.assert_room_membership(player_room, entity);

        let Some(interactable) = interactables.get(entity) else {
            continue;
        };

        let Some(pos) = positions.get(entity).map(|transform| transform.position) else {
            continue;
        };

        let dist = player_pos.distance(pos);
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
    use crate::worlds::room::RoomId;
    use bishop::prelude::Vec2;

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
