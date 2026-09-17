use crate::physics::events::PhysicsEvents;
use crate::physics::sensors::{SensorOverlapTracker, SensorPair};
use std::collections::HashSet;

/// Runtime state retained by the physics simulation across fixed updates.
#[derive(Default)]
pub(crate) struct PhysicsRuntime {
    events: PhysicsEvents,
    sensor_overlaps: SensorOverlapTracker,
}

impl PhysicsRuntime {
    pub(crate) fn events_mut(&mut self) -> &mut PhysicsEvents {
        &mut self.events
    }

    pub(crate) fn finish_sensor_frame(&mut self, current: HashSet<SensorPair>) {
        self.sensor_overlaps.finish_frame(current, &mut self.events);
    }
}
