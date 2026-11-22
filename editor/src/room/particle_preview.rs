use engine_core::ecs::components::{
    CurrentRoom, ParticleConfigIssue, ParticleEmissionMode, ParticleEmitter,
};
use engine_core::ecs::{Ecs, Entity};
use engine_core::rendering::particles::ParticleSystem;

#[derive(Default)]
pub(crate) struct ParticlePreview {
    system: ParticleSystem,
    owner: Option<Entity>,
    active: bool,
    emitting: bool,
    next_preview_sequence: u64,
}

impl ParticlePreview {
    pub(crate) fn burst(
        &mut self,
        owner: Entity,
        ecs: &Ecs,
    ) -> Result<(), Vec<ParticleConfigIssue>> {
        self.validate_owner(owner, ecs)?;
        self.select_owner(owner);
        let configured_seed = ecs
            .get::<ParticleEmitter>(owner)
            .map_or(0, |emitter| emitter.config.seed);
        let preview_seed = self.next_burst_seed(configured_seed);
        self.system.burst_with_seed(owner, ecs, preview_seed)?;
        self.active = !self.system.is_empty();
        self.emitting = false;
        Ok(())
    }

    pub(crate) fn start(
        &mut self,
        owner: Entity,
        ecs: &Ecs,
    ) -> Result<(), Vec<ParticleConfigIssue>> {
        self.validate_owner(owner, ecs)?;
        self.select_owner(owner);
        self.system.start(owner, ecs)?;
        self.active = ecs
            .get::<ParticleEmitter>(owner)
            .is_some_and(|emitter| emitter.config.mode == ParticleEmissionMode::Continuous);
        self.emitting = self.active;
        Ok(())
    }

    pub(crate) fn stop(
        &mut self,
        owner: Entity,
        ecs: &Ecs,
    ) -> Result<(), Vec<ParticleConfigIssue>> {
        self.validate_owner(owner, ecs)?;
        if self.owner != Some(owner) {
            return Ok(());
        }
        self.system.stop(owner, ecs)?;
        self.emitting = false;
        self.active = !self.system.is_empty();
        Ok(())
    }

    pub(crate) fn update(&mut self, dt: f32, selected_owner: Option<Entity>, ecs: &Ecs) {
        if selected_owner != self.owner {
            self.clear();
            self.owner = selected_owner;
        }
        let Some(owner) = self.owner else {
            return;
        };
        if !self.active {
            return;
        }

        self.system.update_emitter(dt, owner, ecs);
        let continuous_mode = ecs
            .get::<ParticleEmitter>(owner)
            .is_some_and(|emitter| emitter.config.mode == ParticleEmissionMode::Continuous);
        if !continuous_mode || !ecs.has::<CurrentRoom>(owner) {
            self.emitting = false;
        }
        if !self.emitting && self.system.is_empty() {
            self.system.remove_emitter(owner);
            self.active = false;
        }
    }

    pub(crate) fn render_system(&self) -> Option<&ParticleSystem> {
        self.active.then_some(&self.system)
    }

    pub(crate) fn is_emitting(&self, owner: Entity) -> bool {
        self.owner == Some(owner) && self.emitting
    }

    pub(crate) fn clear(&mut self) {
        self.system.clear();
        self.owner = None;
        self.active = false;
        self.emitting = false;
    }

    fn validate_owner(
        &self,
        owner: Entity,
        ecs: &Ecs,
    ) -> Result<(), Vec<ParticleConfigIssue>> {
        ecs.get::<ParticleEmitter>(owner)
            .map_or(Ok(()), |emitter| emitter.config.validate())
    }

    fn select_owner(&mut self, owner: Entity) {
        if self.owner != Some(owner) {
            self.clear();
            self.owner = Some(owner);
        }
    }

    fn next_burst_seed(&mut self, configured_seed: u64) -> u64 {
        self.next_preview_sequence = self.next_preview_sequence.wrapping_add(1);
        configured_seed ^ self.next_preview_sequence.wrapping_mul(0x9E37_79B9_7F4A_7C15)
    }

    #[cfg(test)]
    fn is_active(&self) -> bool {
        self.active
    }

    #[cfg(test)]
    fn live_particle_count(&self) -> usize {
        self.system.len()
    }
}

#[cfg(test)]
#[path = "tests/particle_preview_tests.rs"]
mod tests;
