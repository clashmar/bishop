use crate::physics::collision_world::CollisionWorld;
use crate::physics::events::{PhysicsEvents, SensorEvent};
use engine_core::ecs::{
    Active, Collider, CurrentRoom, Ecs, Entity, Kinematic, PhysicsBody, Sensor, SubPixel,
    Transform, true_position,
};
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
        if ecs.has::<Sensor>(entity) {
            continue;
        }
        if !ecs.get::<Active>(entity).is_some_and(Active::is_enabled) {
            continue;
        }
        if !ecs.has::<PhysicsBody>(entity) && !ecs.has::<Kinematic>(entity) {
            continue;
        }
        let Some(current_room) = ecs.get::<CurrentRoom>(entity) else {
            continue;
        };
        if current_room.room_id != room_id {
            continue;
        }
        let Some(transform) = ecs.get::<Transform>(entity).copied() else {
            continue;
        };
        let collider = ecs.get::<Collider>(entity).copied().unwrap_or_default();
        let position = true_position(
            transform.position,
            ecs.get::<SubPixel>(entity).copied().unwrap_or_default(),
        );

        for sensor in collision_world.check_sensor_overlaps(entity, position, collider, transform.pivot) {
            pairs.insert(SensorPair::new(entity, sensor));
        }
    }
}
