use bishop::prelude::Vec2;
use crate::physics::collision_world::shapes_overlap;
use crate::physics::events::{CollisionEvent, PhysicsEvents};
use engine_core::ecs::{
    Active,
    Collider,
    CurrentRoom,
    Ecs,
    Entity,
    Kinematic,
    PhysicsBody,
    Pivot,
    Sensor,
    SubPixel,
    Transform,
    true_position,
};
use engine_core::worlds::{Room, RoomId};
use std::collections::HashSet;

#[derive(Clone, Copy)]
pub(crate) struct CollisionBody {
    pub(crate) entity: Entity,
    pub(crate) position: Vec2,
    pub(crate) collider: Collider,
    pub(crate) pivot: Pivot,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct CollisionPair {
    first: Entity,
    second: Entity,
}

impl CollisionPair {
    pub(crate) fn new(a: Entity, b: Entity) -> Self {
        if a <= b {
            Self { first: a, second: b }
        } else {
            Self { first: b, second: a }
        }
    }
}

#[derive(Default)]
pub(crate) struct CollisionPairTracker {
    previous: HashSet<CollisionPair>,
}

impl CollisionPairTracker {
    pub(crate) fn finish_frame(
        &mut self,
        current: HashSet<CollisionPair>,
        events: &mut PhysicsEvents,
    ) {
        for pair in self.previous.difference(&current) {
            events.push_collision(CollisionEvent::Exit {
                first: pair.first,
                second: pair.second,
            });
        }
        for pair in current.difference(&self.previous) {
            events.push_collision(CollisionEvent::Enter {
                first: pair.first,
                second: pair.second,
            });
        }
        self.previous = current;
    }
}

pub(crate) fn collision_body_in_room(
    ecs: &Ecs,
    room_id: RoomId,
    entity: Entity,
) -> Option<CollisionBody> {
    if ecs.has::<Sensor>(entity) {
        return None;
    }
    if !ecs.get::<Active>(entity).is_some_and(Active::is_enabled) {
        return None;
    }
    if !ecs.has::<PhysicsBody>(entity) && !ecs.has::<Kinematic>(entity) {
        return None;
    }
    if ecs.get::<CurrentRoom>(entity)?.room_id != room_id {
        return None;
    }
    let transform = ecs.get::<Transform>(entity).copied()?;
    let collider = ecs.get::<Collider>(entity).copied().unwrap_or_default();
    let position = true_position(
        transform.position,
        ecs.get::<SubPixel>(entity).copied().unwrap_or_default(),
    );

    Some(CollisionBody {
        entity,
        position,
        collider,
        pivot: transform.pivot,
    })
}

pub(crate) fn collect_collision_pairs(
    ecs: &Ecs,
    room: &Room,
    pairs: &mut HashSet<CollisionPair>,
) {
    let candidates = collision_candidates(ecs, room);
    for (index, first) in candidates.iter().enumerate() {
        for second in candidates.iter().skip(index + 1) {
            if shapes_overlap(
                first.position,
                first.collider,
                first.pivot,
                second.position,
                second.collider,
                second.pivot,
            ) {
                pairs.insert(CollisionPair::new(first.entity, second.entity));
            }
        }
    }
}

fn collision_candidates(ecs: &Ecs, room: &Room) -> Vec<CollisionBody> {
    ecs.entities_in_room(room.id)
        .iter()
        .filter_map(|&entity| collision_body_in_room(ecs, room.id, entity))
        .collect()
}
