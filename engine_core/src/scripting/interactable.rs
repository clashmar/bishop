// engine_core/src/scripting/interactable.rs
use crate::ecs::component::*;
use crate::ecs::ecs::Ecs;
use crate::ecs::entity::Entity;
use crate::ecs::transform::Transform;
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

/// Returns the best interactable entity candidate for the player in the `CurrentRoom` or `None`.
pub fn find_best_interactable(ecs: &Ecs) -> Option<Entity> {
    let player = ecs.get_player_entity()?;
    let player_pos = ecs.get_player_transform()?.position;

    let player_room = ecs.get::<CurrentRoom>(player).map(|r| r.0)?;

    let interactables = ecs.get_store::<Interactable>();
    let positions = ecs.get_store::<Transform>();
    let rooms = ecs.get_store::<CurrentRoom>();

    let mut best: Option<(Entity, f32)> = None;

    for (entity, interactable) in &interactables.data {
        // Must have position and room
        let pos = match positions.get(*entity) {
            Some(p) => p.position,
            None => continue,
        };

        let room = match rooms.get(*entity) {
            Some(r) => r.0,
            None => continue,
        };

        if room != player_room {
            continue;
        }

        // TODO: rethink how position is set for this purpose
        let dist = player_pos.distance(pos);

        if dist > interactable.range {
            continue;
        }

        match best {
            None => best = Some((*entity, dist)),
            Some((_, best_dist)) if dist < best_dist => best = Some((*entity, dist)),
            _ => {}
        }
    }

    best.map(|(entity, _)| entity)
}
