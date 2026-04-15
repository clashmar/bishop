use crate::input::input_constants;
use serde::{Deserialize, Serialize};

/// Canonical built-in profile id for one-second grounded left movement.
pub const BUILTIN_PROFILE_GROUNDED_WALK_LEFT: &str = "grounded_walk_left";

/// Canonical built-in profile id for one-second grounded right movement.
pub const BUILTIN_PROFILE_GROUNDED_WALK_RIGHT: &str = "grounded_walk_right";

/// Canonical built-in profile id for alternating grounded left/right movement.
pub const BUILTIN_PROFILE_MOVEMENT_EVENNESS_LEFT_RIGHT: &str = "movement_evenness_left_right";

/// Canonical built-in profile id for horizontal camera pan sweep coverage.
pub const BUILTIN_PROFILE_CAMERA_PAN_SWEEP: &str = "camera_pan_sweep";

/// Canonical built-in profile id for camera follow toggle coverage.
pub const BUILTIN_PROFILE_CAMERA_FOLLOW_TOGGLE: &str = "camera_follow_toggle";

/// Launch-time request describing which playtest control profile should run.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PlaytestControlRequest {
    /// Profile reference to resolve for this request.
    pub profile: PlaytestControlProfileRef,
    /// Startup behavior to use when this request is installed.
    pub start_policy: ControlStartPolicy,
    /// Repeat behavior to use after the profile reaches the end.
    pub loop_policy: ControlLoopPolicy,
    /// Optional seeded chaos-expansion settings.
    pub chaos: Option<PlaytestChaosConfig>,
}

/// Reference to a concrete control profile.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum PlaytestControlProfileRef {
    /// Uses an inline serialized profile payload.
    Inline(PlaytestControlProfile),
    /// Resolves a profile by name from runtime sources.
    Named(String),
}

/// Serializable movement and camera timeline definition.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct PlaytestControlProfile {
    /// Movement timeline frames for this profile.
    pub movement_frames: Vec<MovementControlFrame>,
    /// Camera timeline frames for this profile.
    pub camera_frames: Vec<CameraControlFrame>,
}

/// One movement-control frame span.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct MovementControlFrame {
    /// Number of engine frames to hold this movement frame.
    pub frame_count: u32,
    /// Inputs that should remain down for this frame span.
    pub down_inputs: Vec<String>,
    /// Inputs that should register as pressed on this frame span.
    pub pressed_inputs: Vec<String>,
    /// Inputs that should register as released on this frame span.
    pub released_inputs: Vec<String>,
}

/// One camera-control frame span.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct CameraControlFrame {
    /// Number of engine frames to hold this camera frame.
    pub frame_count: u32,
    /// Horizontal pan delta to apply for this frame span.
    pub pan_delta_x: f32,
    /// Vertical pan delta to apply for this frame span.
    pub pan_delta_y: f32,
    /// Zoom delta to apply for this frame span.
    pub zoom_delta: f32,
    /// Optional follow-state override for this frame span.
    pub follow_enabled: Option<bool>,
}

/// Startup behavior when a new control request arrives.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ControlStartPolicy {
    /// Replaces any existing active control immediately.
    ReplaceImmediately,
}

/// Repeat behavior for a control request.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ControlLoopPolicy {
    /// Runs the profile once and then stops.
    RunOnce,
    /// Restarts the profile after the last frame.
    Loop,
}

/// Seeded chaos-expansion settings for runtime tasks.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlaytestChaosConfig {
    /// Deterministic seed used when expanding chaos into a concrete profile.
    pub seed: u64,
}

impl PlaytestControlRequest {
    /// Creates a request that resolves a named profile with default policies.
    pub fn named(name: impl Into<String>) -> Self {
        Self {
            profile: PlaytestControlProfileRef::Named(name.into()),
            start_policy: ControlStartPolicy::ReplaceImmediately,
            loop_policy: ControlLoopPolicy::RunOnce,
            chaos: None,
        }
    }
}

impl PlaytestControlProfile {
    /// Resolves a built-in named playtest control profile.
    pub fn builtin_named(name: &str) -> Option<Self> {
        match name {
            BUILTIN_PROFILE_GROUNDED_WALK_LEFT => Some(Self {
                movement_frames: vec![MovementControlFrame {
                    frame_count: 60,
                    down_inputs: vec![input_constants::LEFT.to_string()],
                    pressed_inputs: Vec::new(),
                    released_inputs: Vec::new(),
                }],
                camera_frames: Vec::new(),
            }),
            BUILTIN_PROFILE_GROUNDED_WALK_RIGHT => Some(Self {
                movement_frames: vec![MovementControlFrame {
                    frame_count: 60,
                    down_inputs: vec![input_constants::RIGHT.to_string()],
                    pressed_inputs: Vec::new(),
                    released_inputs: Vec::new(),
                }],
                camera_frames: Vec::new(),
            }),
            BUILTIN_PROFILE_MOVEMENT_EVENNESS_LEFT_RIGHT => Some(Self {
                movement_frames: vec![
                    MovementControlFrame {
                        frame_count: 30,
                        down_inputs: vec![input_constants::LEFT.to_string()],
                        pressed_inputs: Vec::new(),
                        released_inputs: Vec::new(),
                    },
                    MovementControlFrame {
                        frame_count: 30,
                        down_inputs: vec![input_constants::RIGHT.to_string()],
                        pressed_inputs: Vec::new(),
                        released_inputs: Vec::new(),
                    },
                ],
                camera_frames: Vec::new(),
            }),
            BUILTIN_PROFILE_CAMERA_PAN_SWEEP => Some(Self {
                movement_frames: Vec::new(),
                camera_frames: vec![
                    CameraControlFrame {
                        frame_count: 30,
                        pan_delta_x: -2.0,
                        pan_delta_y: 0.0,
                        zoom_delta: 0.0,
                        follow_enabled: None,
                    },
                    CameraControlFrame {
                        frame_count: 60,
                        pan_delta_x: 2.0,
                        pan_delta_y: 0.0,
                        zoom_delta: 0.0,
                        follow_enabled: None,
                    },
                    CameraControlFrame {
                        frame_count: 30,
                        pan_delta_x: -2.0,
                        pan_delta_y: 0.0,
                        zoom_delta: 0.0,
                        follow_enabled: None,
                    },
                ],
            }),
            BUILTIN_PROFILE_CAMERA_FOLLOW_TOGGLE => Some(Self {
                movement_frames: Vec::new(),
                camera_frames: vec![
                    CameraControlFrame {
                        frame_count: 30,
                        pan_delta_x: 0.0,
                        pan_delta_y: 0.0,
                        zoom_delta: 0.0,
                        follow_enabled: Some(false),
                    },
                    CameraControlFrame {
                        frame_count: 30,
                        pan_delta_x: 0.0,
                        pan_delta_y: 0.0,
                        zoom_delta: 0.0,
                        follow_enabled: Some(true),
                    },
                ],
            }),
            _ => None,
        }
    }
}
