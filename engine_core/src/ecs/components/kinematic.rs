use bishop::prelude::Vec2;
use ecs_component::ecs_component;
use serde::{Deserialize, Serialize};
use strum_macros::EnumIter;

use super::{Active, Collider, Transform, Velocity};

/// Configures how a kinematic body responds when it contacts other bodies.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, EnumIter)]
pub enum KinematicContactBehavior {
    #[default]
    Stop,
    Crush,
    Eject,
    Reverse,
    Trigger,
}

/// Authored motion mode for a kinematic body.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, EnumIter)]
pub enum KinematicMotionMode {
    #[default]
    None,
    Constant,
    PingPong,
}

/// Primary movement axis for authored kinematic motion.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, EnumIter)]
pub enum KinematicAxis {
    #[default]
    Horizontal,
    Vertical,
}

/// Initial authored direction for a kinematic body.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, EnumIter)]
pub enum KinematicDirection {
    #[default]
    Positive,
    Negative,
}

impl KinematicContactBehavior {
    /// Returns the UI label for this contact behavior.
    pub fn label(self) -> &'static str {
        match self {
            Self::Stop => "Stop",
            Self::Crush => "Crush",
            Self::Eject => "Eject",
            Self::Reverse => "Reverse",
            Self::Trigger => "Trigger",
        }
    }

    /// Returns whether this behavior should act as a solid obstacle.
    pub fn is_solid(self) -> bool {
        !matches!(self, Self::Trigger)
    }

    /// Returns whether this behavior requires ping-pong motion.
    pub fn requires_ping_pong(self) -> bool {
        matches!(self, Self::Reverse)
    }
}

impl std::fmt::Display for KinematicContactBehavior {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}

impl KinematicMotionMode {
    /// Returns the UI label for this kinematic motion mode.
    pub fn label(self) -> &'static str {
        match self {
            Self::None => "None",
            Self::Constant => "Constant",
            Self::PingPong => "Ping-Pong",
        }
    }
}

impl std::fmt::Display for KinematicMotionMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}

impl KinematicAxis {
    /// Returns the UI label for this kinematic motion axis.
    pub fn label(self) -> &'static str {
        match self {
            Self::Horizontal => "Horizontal",
            Self::Vertical => "Vertical",
        }
    }
}

impl std::fmt::Display for KinematicAxis {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}

impl KinematicDirection {
    /// Returns the UI label for this authored direction.
    pub fn label(self) -> &'static str {
        match self {
            Self::Positive => "Positive",
            Self::Negative => "Negative",
        }
    }

    /// Returns the signed scalar for this authored direction.
    pub fn sign(self) -> f32 {
        match self {
            Self::Positive => 1.0,
            Self::Negative => -1.0,
        }
    }

    /// Returns the opposite authored direction.
    pub fn reversed(self) -> Self {
        match self {
            Self::Positive => Self::Negative,
            Self::Negative => Self::Positive,
        }
    }
}

impl std::fmt::Display for KinematicDirection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}

/// Designer-authored movement settings for a kinematic body.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct KinematicMotion {
    pub mode: KinematicMotionMode,
    pub axis: KinematicAxis,
    pub direction: KinematicDirection,
    pub speed: f32,
    pub travel_distance: f32,
}

impl Default for KinematicMotion {
    fn default() -> Self {
        Self {
            mode: KinematicMotionMode::None,
            axis: KinematicAxis::Horizontal,
            direction: KinematicDirection::Positive,
            speed: 60.0,
            travel_distance: 64.0,
        }
    }
}

/// Marks a moving solid body authored in-engine.
#[ecs_component(deps = [Active, Collider, Transform, Velocity])]
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Kinematic {
    pub contact_behavior: KinematicContactBehavior,
    pub motion: KinematicMotion,
    #[serde(skip)]
    runtime_origin_x: f32,
    #[serde(skip)]
    runtime_origin_y: f32,
    #[serde(skip)]
    runtime_has_origin: bool,
    #[serde(skip)]
    runtime_direction: KinematicDirection,
}

impl Default for Kinematic {
    fn default() -> Self {
        Self {
            contact_behavior: KinematicContactBehavior::Stop,
            motion: KinematicMotion::default(),
            runtime_origin_x: 0.0,
            runtime_origin_y: 0.0,
            runtime_has_origin: false,
            runtime_direction: KinematicDirection::Positive,
        }
    }
}

impl Kinematic {
    /// Clears authored motion runtime state.
    pub fn clear_runtime_state(&mut self) {
        self.runtime_origin_x = 0.0;
        self.runtime_origin_y = 0.0;
        self.runtime_has_origin = false;
        self.runtime_direction = self.motion.direction;
    }

    /// Returns the current authored motion origin.
    pub fn runtime_origin(&self) -> Option<Vec2> {
        self.runtime_has_origin
            .then(|| Vec2::new(self.runtime_origin_x, self.runtime_origin_y))
    }

    /// Stores the current authored motion origin.
    pub fn set_runtime_origin(&mut self, origin: Vec2) {
        self.runtime_origin_x = origin.x;
        self.runtime_origin_y = origin.y;
        self.runtime_has_origin = true;
    }

    /// Returns the current runtime travel direction.
    pub fn runtime_direction(&self) -> KinematicDirection {
        self.runtime_direction
    }

    /// Stores the current runtime travel direction.
    pub fn set_runtime_direction(&mut self, direction: KinematicDirection) {
        self.runtime_direction = direction;
    }
}
