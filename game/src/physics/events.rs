use engine_core::ecs::Entity;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum KinematicContactEvent {
    Contact { kinematic: Entity, dynamic: Entity },
    Crushed { kinematic: Entity, dynamic: Entity },
}

impl KinematicContactEvent {
    /// Returns the kinematic entity involved in this contact event.
    pub(crate) fn kinematic(self) -> Entity {
        match self {
            Self::Contact { kinematic, .. } | Self::Crushed { kinematic, .. } => kinematic,
        }
    }

    /// Returns the non-kinematic entity involved in this contact event.
    pub(crate) fn other(self) -> Entity {
        match self {
            Self::Contact { dynamic, .. } | Self::Crushed { dynamic, .. } => dynamic,
        }
    }
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
