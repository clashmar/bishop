use crate::ecs::components::{
    CurrentRoom, ParticleConfigIssue, ParticleEmissionMode, ParticleEmitter, ParticleEmitterConfig,
    ParticleVisual,
};
use crate::ecs::{Ecs, Entity, Transform};
use crate::worlds::room::RoomId;
use bishop::types::{Color, Vec2};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use std::collections::{HashMap, HashSet};

/// Maximum number of live particles shared by one particle system.
pub const PARTICLE_POOL_CAPACITY: usize = 4096;

/// Runtime state for one emitted particle.
#[derive(Clone, Debug, PartialEq)]
pub struct Particle {
    /// Entity that owns this particle's room layer and z ordering.
    pub owner: Entity,
    /// World-space position captured and advanced after emission.
    pub position: Vec2,
    /// World-space velocity sampled at emission.
    pub velocity: Vec2,
    /// Elapsed particle lifetime in seconds.
    pub age: f32,
    /// Total particle lifetime in seconds.
    pub lifetime: f32,
    /// Interpolated visual scale multiplier.
    pub scale: f32,
    scale_start: f32,
    scale_end: f32,
    /// Interpolated opacity multiplier in 0..=1.
    pub opacity: f32,
    opacity_start: f32,
    opacity_end: f32,
    /// Particle tint; renderers multiply `opacity` into its alpha.
    pub color: Color,
    /// Visual captured when the particle was emitted.
    pub visual: ParticleVisual,
    /// Stable system-wide ordering key assigned at emission.
    pub emission_sequence: u64,
}

struct EmitterRuntimeState {
    rng: StdRng,
    seed: u64,
    active: bool,
    emission_accumulator: f32,
    active_particles: usize,
    room_id: RoomId,
}

/// Fixed-capacity runtime simulation for entity-owned particle emitters.
pub struct ParticleSystem {
    slots: Vec<Option<Particle>>,
    free_slots: Vec<usize>,
    emitter_states: HashMap<Entity, EmitterRuntimeState>,
    next_emission_sequence: u64,
}

impl ParticleSystem {
    /// Creates a particle system with the engine-wide pool capacity.
    pub fn new() -> Self {
        Self::with_capacity(PARTICLE_POOL_CAPACITY)
    }

    /// Returns the configured number of slots in this system's pool.
    pub fn capacity(&self) -> usize {
        self.slots.len()
    }

    /// Returns the number of currently live particles.
    pub fn len(&self) -> usize {
        self.slots.len() - self.free_slots.len()
    }

    /// Returns whether the system has no live particles.
    pub fn is_empty(&self) -> bool {
        self.free_slots.len() == self.slots.len()
    }

    /// Returns a read-only iterator over live particles.
    pub fn particles(&self) -> impl Iterator<Item = &Particle> {
        self.slots.iter().filter_map(Option::as_ref)
    }

    /// Emits a burst for a burst-mode owner; mode-inapplicable requests do nothing.
    pub fn burst(
        &mut self,
        owner: Entity,
        ecs: &Ecs,
    ) -> Result<(), Vec<ParticleConfigIssue>> {
        let Some((config, room_id, position)) = emitter_snapshot(owner, ecs) else {
            return Ok(());
        };
        config.validate()?;
        if config.mode != ParticleEmissionMode::Burst {
            return Ok(());
        }

        self.prepare_emitter(owner, &config, room_id);
        if let Some(position) = position {
            self.emit(owner, &config, position, config.burst_count as usize);
        }
        Ok(())
    }

    /// Starts a continuous-mode owner and clears its fractional emission timer.
    pub fn start(
        &mut self,
        owner: Entity,
        ecs: &Ecs,
    ) -> Result<(), Vec<ParticleConfigIssue>> {
        let Some((config, room_id, _)) = emitter_snapshot(owner, ecs) else {
            return Ok(());
        };
        config.validate()?;
        if config.mode != ParticleEmissionMode::Continuous {
            return Ok(());
        }

        self.prepare_emitter(owner, &config, room_id);
        if let Some(state) = self.emitter_states.get_mut(&owner) {
            state.active = true;
            state.emission_accumulator = 0.0;
        }
        Ok(())
    }

    /// Stops continuous emission without removing already emitted particles.
    pub fn stop(
        &mut self,
        owner: Entity,
        ecs: &Ecs,
    ) -> Result<(), Vec<ParticleConfigIssue>> {
        let Some((config, room_id, _)) = emitter_snapshot(owner, ecs) else {
            return Ok(());
        };
        config.validate()?;
        if config.mode != ParticleEmissionMode::Continuous {
            return Ok(());
        }

        self.prepare_emitter(owner, &config, room_id);
        if let Some(state) = self.emitter_states.get_mut(&owner) {
            state.active = false;
            state.emission_accumulator = 0.0;
        }
        Ok(())
    }

    /// Replaces an owner's shared configuration after validating it atomically.
    pub fn configure(
        &mut self,
        owner: Entity,
        config: ParticleEmitterConfig,
        ecs: &mut Ecs,
    ) -> Result<(), Vec<ParticleConfigIssue>> {
        config.validate()?;
        let Some(emitter) = ecs.get_mut::<ParticleEmitter>(owner) else {
            return Ok(());
        };
        let previous_mode = emitter.config.mode;
        let previous_seed = emitter.config.seed;
        emitter.config = config.clone();

        if let Some(state) = self.emitter_states.get_mut(&owner) {
            if previous_seed != config.seed {
                state.seed = config.seed;
                state.rng = StdRng::seed_from_u64(config.seed);
            }
            if previous_mode != config.mode {
                state.active = config.mode == ParticleEmissionMode::Continuous;
                state.emission_accumulator = 0.0;
            }
        }
        Ok(())
    }

    /// Advances particles, emits due continuous particles, and removes stale owners.
    pub fn update(&mut self, dt: f32, ecs: &Ecs) {
        let dt = if dt.is_finite() && dt > 0.0 { dt } else { 0.0 };
        let emitters = ecs
            .get_store::<ParticleEmitter>()
            .data
            .iter()
            .filter_map(|(owner, emitter)| {
                let current_room = ecs.get::<CurrentRoom>(*owner)?;
                let position = ecs.get::<Transform>(*owner).map(|transform| transform.position);
                Some((*owner, emitter.config.clone(), current_room.room_id, position))
            })
            .collect::<Vec<_>>();
        let emitter_ids = emitters
            .iter()
            .map(|(owner, _, _, _)| *owner)
            .collect::<HashSet<_>>();
        let stale_owners = self
            .emitter_states
            .keys()
            .copied()
            .filter(|owner| !emitter_ids.contains(owner))
            .collect::<Vec<_>>();

        for owner in stale_owners {
            self.remove_emitter(owner);
        }

        for (owner, config, room_id, _) in &emitters {
            self.prepare_emitter(*owner, config, *room_id);
        }

        if dt > 0.0 {
            self.advance_particles(dt);
        }

        for (owner, config, _, position) in emitters {
            if config.mode != ParticleEmissionMode::Continuous || config.validate().is_err() {
                continue;
            }
            let Some(position) = position else {
                continue;
            };
            let due = self.accumulate_continuous_emission(owner, dt, config.emission_rate);
            self.emit(owner, &config, position, due);
        }
    }

    /// Removes one owner's particles and transient emission state.
    pub fn remove_emitter(&mut self, owner: Entity) {
        self.release_owner_particles(owner);
        self.emitter_states.remove(&owner);
    }

    /// Clears all live particles and per-emitter runtime state.
    pub fn clear(&mut self) {
        for slot in &mut self.slots {
            *slot = None;
        }
        let slot_count = self.slots.len();
        self.free_slots.clear();
        self.free_slots.extend((0..slot_count).rev());
        self.emitter_states.clear();
        self.next_emission_sequence = 0;
    }

    fn prepare_emitter(
        &mut self,
        owner: Entity,
        config: &ParticleEmitterConfig,
        room_id: RoomId,
    ) {
        let room_changed = self
            .emitter_states
            .get(&owner)
            .is_some_and(|state| state.room_id != room_id);
        if room_changed {
            self.release_owner_particles(owner);
        }

        let state = self.emitter_states.entry(owner).or_insert_with(|| EmitterRuntimeState {
            rng: StdRng::seed_from_u64(config.seed),
            seed: config.seed,
            active: config.mode == ParticleEmissionMode::Continuous,
            emission_accumulator: 0.0,
            active_particles: 0,
            room_id,
        });
        if state.seed != config.seed {
            state.seed = config.seed;
            state.rng = StdRng::seed_from_u64(config.seed);
        }
        if state.room_id != room_id {
            state.room_id = room_id;
            state.emission_accumulator = 0.0;
        }
    }

    fn accumulate_continuous_emission(
        &mut self,
        owner: Entity,
        dt: f32,
        emission_rate: f32,
    ) -> usize {
        let Some(state) = self.emitter_states.get_mut(&owner) else {
            return 0;
        };
        if !state.active {
            return 0;
        }

        let total = state.emission_accumulator + dt * emission_rate;
        if !total.is_finite() {
            state.emission_accumulator = 0.0;
            return usize::MAX;
        }
        let due = total.floor() as usize;
        state.emission_accumulator = total.fract();
        due
    }

    fn emit(
        &mut self,
        owner: Entity,
        config: &ParticleEmitterConfig,
        origin: Vec2,
        requested: usize,
    ) {
        for _ in 0..requested {
            let Some(slot_index) = self.free_slots.pop() else {
                break;
            };
            let Some(state) = self.emitter_states.get_mut(&owner) else {
                self.free_slots.push(slot_index);
                break;
            };
            if state.active_particles >= config.max_particles {
                self.free_slots.push(slot_index);
                break;
            }

            let half_area = config.area_size * 0.5;
            let position = origin
                + Vec2::new(
                    state.rng.gen_range(-half_area.x..=half_area.x),
                    state.rng.gen_range(-half_area.y..=half_area.y),
                );
            let velocity = Vec2::new(
                state
                    .rng
                    .gen_range(config.velocity_min.x..=config.velocity_max.x),
                state
                    .rng
                    .gen_range(config.velocity_min.y..=config.velocity_max.y),
            );
            let lifetime = state
                .rng
                .gen_range(config.lifetime_min..=config.lifetime_max);
            let emission_sequence = self.next_emission_sequence;
            self.next_emission_sequence = self.next_emission_sequence.wrapping_add(1);
            state.active_particles += 1;
            self.slots[slot_index] = Some(Particle {
                owner,
                position,
                velocity,
                age: 0.0,
                lifetime,
                scale: config.scale_start,
                scale_start: config.scale_start,
                scale_end: config.scale_end,
                opacity: config.opacity_start,
                opacity_start: config.opacity_start,
                opacity_end: config.opacity_end,
                color: config.color,
                visual: config.visual.clone(),
                emission_sequence,
            });
        }
    }

    fn advance_particles(&mut self, dt: f32) {
        let (slots, free_slots, emitter_states) =
            (&mut self.slots, &mut self.free_slots, &mut self.emitter_states);
        for (slot_index, slot) in slots.iter_mut().enumerate() {
            let Some(particle) = slot.as_mut() else {
                continue;
            };
            particle.age += dt;
            if particle.age >= particle.lifetime {
                if let Some(state) = emitter_states.get_mut(&particle.owner) {
                    state.active_particles = state.active_particles.saturating_sub(1);
                }
                *slot = None;
                free_slots.push(slot_index);
                continue;
            }

            particle.position += particle.velocity * dt;
            let lifetime_progress = (particle.age / particle.lifetime).clamp(0.0, 1.0);
            particle.scale = interpolate(particle.scale_start, particle.scale_end, lifetime_progress);
            particle.opacity = interpolate(particle.opacity_start, particle.opacity_end, lifetime_progress);
        }
    }

    fn release_owner_particles(&mut self, owner: Entity) {
        let mut released = 0;
        let (slots, free_slots) = (&mut self.slots, &mut self.free_slots);
        for (slot_index, slot) in slots.iter_mut().enumerate() {
            if slot
                .as_ref()
                .is_some_and(|particle| particle.owner == owner)
            {
                *slot = None;
                free_slots.push(slot_index);
                released += 1;
            }
        }
        if let Some(state) = self.emitter_states.get_mut(&owner) {
            state.active_particles = state.active_particles.saturating_sub(released);
        }
    }

    /// Creates a particle system with a pool capacity for internal use.
    pub(super) fn with_capacity(capacity: usize) -> Self {
        let mut free_slots = Vec::with_capacity(capacity);
        free_slots.extend((0..capacity).rev());
        Self {
            slots: vec![None; capacity],
            free_slots,
            emitter_states: HashMap::new(),
            next_emission_sequence: 0,
        }
    }
}

impl Default for ParticleSystem {
    fn default() -> Self {
        Self::new()
    }
}

fn emitter_snapshot(
    owner: Entity,
    ecs: &Ecs,
) -> Option<(ParticleEmitterConfig, RoomId, Option<Vec2>)> {
    let config = ecs.get::<ParticleEmitter>(owner)?.config.clone();
    let room_id = ecs.get::<CurrentRoom>(owner)?.room_id;
    let position = ecs.get::<Transform>(owner).map(|transform| transform.position);
    Some((config, room_id, position))
}

fn interpolate(start: f32, end: f32, progress: f32) -> f32 {
    start + (end - start) * progress
}


