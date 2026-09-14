use crate::engine::Engine;
use crate::scripting::commands::lua_command::LuaCommand;
use engine_core::ecs::{Entity, Kinematic, KinematicAxis, KinematicDirection, KinematicMotionMode};
use engine_core::logging::omni_error;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum KinematicCommandKind {
    Start,
    Stop,
    Reverse,
    SetEnabled(bool),
    SetMode(KinematicMotionMode),
    SetAxis(KinematicAxis),
    SetDirection(KinematicDirection),
    SetSpeed(f32),
    SetTravelDistance(f32),
}

pub(crate) struct SetKinematicCmd {
    pub(crate) entity: Entity,
    pub(crate) kind: KinematicCommandKind,
}

impl LuaCommand for SetKinematicCmd {
    fn execute(&mut self, engine: &mut Engine) {
        let mut game_instance = engine.game_instance.borrow_mut();
        let ecs = &mut game_instance.game.ecs;
        let Some(kinematic) = ecs.get_mut::<Kinematic>(self.entity) else {
            omni_error!("Kinematic command target {:?} is missing Kinematic", self.entity);
            return;
        };

        if let Err(error) = apply_kinematic_command(kinematic, self.kind) {
            omni_error!("{} for {:?}", error, self.entity);
        }
    }
}

fn apply_kinematic_command(
    kinematic: &mut Kinematic,
    kind: KinematicCommandKind,
) -> Result<(), &'static str> {
    match kind {
        KinematicCommandKind::Start => kinematic.start_runtime(),
        KinematicCommandKind::Stop => kinematic.stop_runtime(),
        KinematicCommandKind::Reverse => {
            if kinematic.motion.mode != KinematicMotionMode::PingPong {
                return Err("reverse kinematic command requires ping-pong motion");
            }
            kinematic.set_runtime_direction(kinematic.runtime_direction().reversed());
        }
        KinematicCommandKind::SetEnabled(enabled) => kinematic.set_runtime_enabled(enabled),
        KinematicCommandKind::SetMode(mode) => {
            kinematic.motion.mode = mode;
            reset_motion_runtime_state(kinematic);
        }
        KinematicCommandKind::SetAxis(axis) => {
            kinematic.motion.axis = axis;
            reset_motion_runtime_state(kinematic);
        }
        KinematicCommandKind::SetDirection(direction) => {
            kinematic.motion.direction = direction;
            reset_motion_runtime_state(kinematic);
        }
        KinematicCommandKind::SetSpeed(speed) => kinematic.motion.speed = speed,
        KinematicCommandKind::SetTravelDistance(distance) => {
            kinematic.motion.travel_distance = distance;
        }
    }

    Ok(())
}

fn reset_motion_runtime_state(kinematic: &mut Kinematic) {
    let was_enabled = kinematic.is_runtime_enabled();
    let was_running = kinematic.is_runtime_running();

    kinematic.clear_runtime_state();

    if !was_enabled {
        kinematic.set_runtime_enabled(false);
    } else if !was_running {
        kinematic.stop_runtime();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bishop::Vec2;
    use engine_core::ecs::KinematicMotion;

    #[test]
    fn set_enabled_command_disables_runtime_motion() {
        let mut kinematic = Kinematic::default();

        apply_kinematic_command(&mut kinematic, KinematicCommandKind::SetEnabled(false)).unwrap();

        assert!(!kinematic.is_runtime_enabled());
        assert!(!kinematic.is_runtime_running());
    }

    #[test]
    fn motion_edit_commands_reset_runtime_state_and_preserve_state_flags() {
        let mut kinematic = Kinematic::default();
        kinematic.motion = KinematicMotion {
            mode: KinematicMotionMode::PingPong,
            axis: KinematicAxis::Horizontal,
            direction: KinematicDirection::Negative,
            speed: 24.0,
            travel_distance: 48.0,
        };
        kinematic.set_runtime_origin(Vec2::new(9.0, 5.0));
        kinematic.set_runtime_direction(KinematicDirection::Positive);
        kinematic.stop_runtime();

        apply_kinematic_command(
            &mut kinematic,
            KinematicCommandKind::SetMode(KinematicMotionMode::Constant),
        )
        .unwrap();

        assert_eq!(kinematic.motion.mode, KinematicMotionMode::Constant);
        assert_eq!(kinematic.runtime_origin(), None);
        assert_eq!(kinematic.runtime_direction(), KinematicDirection::Negative);
        assert!(kinematic.is_runtime_enabled());
        assert!(!kinematic.is_runtime_running());

        kinematic.set_runtime_origin(Vec2::new(3.0, 7.0));
        kinematic.set_runtime_direction(KinematicDirection::Positive);
        kinematic.set_runtime_enabled(false);

        apply_kinematic_command(
            &mut kinematic,
            KinematicCommandKind::SetAxis(KinematicAxis::Vertical),
        )
        .unwrap();

        assert_eq!(kinematic.motion.axis, KinematicAxis::Vertical);
        assert_eq!(kinematic.runtime_origin(), None);
        assert_eq!(kinematic.runtime_direction(), KinematicDirection::Negative);
        assert!(!kinematic.is_runtime_enabled());
        assert!(!kinematic.is_runtime_running());
    }
}
