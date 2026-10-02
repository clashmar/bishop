use crate::physics::collisions::collision_body_in_room;
use crate::physics::collision_world::CollisionWorld;
use crate::physics::events::{PhysicsEvents, SensorEvent};
use engine_core::ecs::{Ecs, Entity};
use engine_core::worlds::RoomId;
use std::collections::HashSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct SensorPair {
    body: Entity,
    sensor: Entity,
}

impl SensorPair {
    fn new(body: Entity, sensor: Entity) -> Self {
        Self { body, sensor }
    }
}

#[derive(Default)]
pub(crate) struct SensorOverlapTracker {
    previous: HashSet<SensorPair>,
}

impl SensorOverlapTracker {
    pub(crate) fn finish_frame(
        &mut self,
        current: HashSet<SensorPair>,
        events: &mut PhysicsEvents,
    ) {
        for pair in self.previous.difference(&current) {
            events.push_sensor(SensorEvent::Exit {
                body: pair.body,
                sensor: pair.sensor,
            });
        }
        for pair in current.difference(&self.previous) {
            events.push_sensor(SensorEvent::Enter {
                body: pair.body,
                sensor: pair.sensor,
            });
        }
        for pair in current.intersection(&self.previous) {
            events.push_sensor(SensorEvent::Stay {
                body: pair.body,
                sensor: pair.sensor,
            });
        }
        self.previous = current;
    }
}

pub(crate) fn collect_sensor_pairs(
    ecs: &Ecs,
    room_id: RoomId,
    collision_world: &CollisionWorld,
    pairs: &mut HashSet<SensorPair>,
) {
    for &entity in ecs.entities_in_room(room_id) {
        let Some(body) = collision_body_in_room(ecs, room_id, entity) else {
            continue;
        };

        for sensor in collision_world.check_sensor_overlaps(
            body.entity,
            body.position,
            body.collider,
            body.pivot,
        ) {
            pairs.insert(SensorPair::new(body.entity, sensor));
        }
    }
}
