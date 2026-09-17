use engine_core::ecs::{
    KinematicAxis, KinematicDirection, KinematicMotion, KinematicMotionMode,
};

pub(crate) const DT: f32 = 1.0 / 60.0;
pub(crate) const GRID_SIZE: f32 = 16.0;

/// Creates constant horizontal kinematic motion for physics tests.
pub(crate) fn horizontal_constant(speed: f32) -> KinematicMotion {
    KinematicMotion {
        mode: KinematicMotionMode::Constant,
        axis: KinematicAxis::Horizontal,
        direction: if speed >= 0.0 {
            KinematicDirection::Positive
        } else {
            KinematicDirection::Negative
        },
        speed: speed.abs(),
        travel_distance: 0.0,
    }
}
