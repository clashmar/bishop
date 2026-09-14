use bishop::prelude::Vec2;
use ecs_component::ecs_component;
use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumIter, EnumProperty, EnumString};

use crate::ecs::{Ecs, Entity};

use super::{Active, Collider, Transform, Velocity};

/// Configures how a kinematic body responds when it contacts other bodies.
#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    Display,
    EnumIter,
    EnumProperty,
    EnumString,
)]
pub enum KinematicContactBehavior {
    #[default]
    #[strum(props(lua = "stop"))]
    Stop,
    #[strum(props(lua = "crush"))]
    Crush,
    #[strum(props(lua = "eject"))]
    Eject,
    #[strum(props(lua = "reverse"))]
    Reverse,
    #[strum(props(lua = "trigger"))]
    Trigger,
}

/// Authored motion mode for a kinematic body.
#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    Display,
    EnumIter,
    EnumProperty,
    EnumString,
)]
pub enum KinematicMotionMode {
    #[default]
    #[strum(serialize = "none", to_string = "None", props(lua = "none"))]
    None,
    #[strum(serialize = "constant", to_string = "Constant", props(lua = "constant"))]
    Constant,
    #[strum(serialize = "ping_pong", to_string = "Ping-Pong", props(lua = "ping_pong"))]
    PingPong,
}

/// Primary movement axis for authored kinematic motion.
#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    Display,
    EnumIter,
    EnumProperty,
    EnumString,
)]
pub enum KinematicAxis {
    #[default]
    #[strum(serialize = "horizontal", to_string = "Horizontal", props(lua = "horizontal"))]
    Horizontal,
    #[strum(serialize = "vertical", to_string = "Vertical", props(lua = "vertical"))]
    Vertical,
}

/// Initial authored direction for a kinematic body.
#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    Display,
    EnumIter,
    EnumProperty,
    EnumString,
)]
pub enum KinematicDirection {
    #[default]
    #[strum(serialize = "positive", to_string = "Positive", props(lua = "positive"))]
    Positive,
    #[strum(serialize = "negative", to_string = "Negative", props(lua = "negative"))]
    Negative,
}

impl KinematicContactBehavior {
    /// Returns whether this behavior should act as a solid obstacle.
    pub fn is_solid(self) -> bool {
        !matches!(self, Self::Trigger)
    }

    /// Returns whether this behavior requires ping-pong motion.
    pub fn requires_ping_pong(self) -> bool {
        matches!(self, Self::Reverse)
    }
}

impl KinematicDirection {
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
#[ecs_component(on_insert = on_insert, deps = [Active, Collider, Transform, Velocity])]
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
    #[serde(skip)]
    runtime_enabled: bool,
    #[serde(skip)]
    runtime_running: bool,
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
            runtime_enabled: true,
            runtime_running: true,
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
        self.runtime_enabled = true;
        self.runtime_running = true;
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

    /// Starts runtime movement using the current authored motion.
    pub fn start_runtime(&mut self) {
        self.runtime_enabled = true;
        self.runtime_running = true;
    }

    /// Stops runtime movement without disabling the kinematic.
    pub fn stop_runtime(&mut self) {
        self.runtime_running = false;
    }

    /// Enables or disables the kinematic for runtime motion and contact handling.
    pub fn set_runtime_enabled(&mut self, enabled: bool) {
        self.runtime_enabled = enabled;
        if !enabled {
            self.runtime_running = false;
        }
    }

    /// Returns whether this kinematic is enabled for runtime simulation.
    pub fn is_runtime_enabled(&self) -> bool {
        self.runtime_enabled
    }

    /// Returns whether this kinematic is actively moving at runtime.
    pub fn is_runtime_running(&self) -> bool {
        self.runtime_enabled && self.runtime_running
    }
}

fn on_insert(kinematic: &mut Kinematic, _entity: &Entity, _ecs: &mut Ecs) {
    kinematic.clear_runtime_state();
}
