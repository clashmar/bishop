use engine_core::ecs::Entity;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum KinematicContactEvent {
    Contact { kinematic: Entity, dynamic: Entity },
    Crushed { kinematic: Entity, dynamic: Entity },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PhysicsEvent {
    KinematicContact(KinematicContactEvent),
}

#[derive(Default)]
pub(crate) struct PhysicsEvents {
    queued: Vec<PhysicsEvent>,
}

impl PhysicsEvents {
    pub(crate) fn push_kinematic_contact(&mut self, event: KinematicContactEvent) {
        self.queued.push(PhysicsEvent::KinematicContact(event));
    }

    pub(crate) fn drain(&mut self) -> Vec<PhysicsEvent> {
        std::mem::take(&mut self.queued)
    }
}
