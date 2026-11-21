use crate::engine::Engine;
use crate::scripting::commands::lua_command::LuaCommand;
use engine_core::ecs::{Ecs, Entity, ParticleConfigIssue, ParticleEmitterConfig};
use engine_core::logging::omni_error;
use engine_core::rendering::ParticleSystem;

pub(crate) struct ParticleEmitterCommand {
    pub(crate) owner: Entity,
    pub(crate) action: Option<ParticleEmitterAction>,
}

pub(crate) enum ParticleEmitterAction {
    Configure(ParticleEmitterConfig),
    Burst,
    Start,
    Stop,
}

impl ParticleEmitterCommand {
    fn apply(
        &mut self,
        particle_system: &mut ParticleSystem,
        ecs: &mut Ecs,
    ) -> Result<(), Vec<ParticleConfigIssue>> {
        let Some(action) = self.action.take() else {
            return Ok(());
        };
        match action {
            ParticleEmitterAction::Configure(config) => {
                particle_system.configure(self.owner, config, ecs)
            }
            ParticleEmitterAction::Burst => particle_system.burst(self.owner, ecs),
            ParticleEmitterAction::Start => particle_system.start(self.owner, ecs),
            ParticleEmitterAction::Stop => particle_system.stop(self.owner, ecs),
        }
    }
}

impl LuaCommand for ParticleEmitterCommand {
    fn execute(&mut self, engine: &mut Engine) {
        let result = {
            let mut game_instance = engine.game_instance.borrow_mut();
            self.apply(&mut engine.particle_system, &mut game_instance.game.ecs)
        };
        if let Err(issues) = result {
            omni_error!(
                "{}",
                crate::scripting::modules::particle_module::format_particle_config_issues(
                    &issues,
                )
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bishop::types::Vec2;
    use engine_core::ecs::{Active, ParticleEmissionMode, ParticleEmitter, Transform};
    use engine_core::worlds::{RoomId, RoomLayer};

    #[test]
    fn apply_orders_configuration_and_continuous_lifecycle_actions() {
        let mut ecs = Ecs::default();
        let owner = ecs
            .create_entity()
            .with(Active::default())
            .with(Transform {
                position: Vec2::ZERO,
                ..Transform::default()
            })
            .with(ParticleEmitter::default())
            .with_current_room_layer(RoomId(1), RoomLayer::Front)
            .finish();
        let config = ParticleEmitterConfig {
            mode: ParticleEmissionMode::Continuous,
            lifetime_min: 10.0,
            lifetime_max: 10.0,
            emission_rate: 4.0,
            ..ParticleEmitterConfig::default()
        };
        let mut system = ParticleSystem::new();

        let mut configure = ParticleEmitterCommand {
            owner,
            action: Some(ParticleEmitterAction::Configure(config.clone())),
        };
        configure.apply(&mut system, &mut ecs).expect("config should apply");
        let mut start = ParticleEmitterCommand {
            owner,
            action: Some(ParticleEmitterAction::Start),
        };
        start.apply(&mut system, &mut ecs).expect("start should apply");
        system.update(0.5, &ecs);
        assert_eq!(system.len(), 2);

        let mut stop = ParticleEmitterCommand {
            owner,
            action: Some(ParticleEmitterAction::Stop),
        };
        stop.apply(&mut system, &mut ecs).expect("stop should apply");
        system.update(0.5, &ecs);
        assert_eq!(system.len(), 2);
        assert_eq!(
            ecs.get::<ParticleEmitter>(owner)
                .expect("emitter should retain its configuration")
                .config,
            config,
        );
    }
}
