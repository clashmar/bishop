use crate::ecs::components::current_frame::CurrentFrameSnapshot;
use bishop::types::{Color, Vec2};
use ecs_component::ecs_component;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use strum::IntoEnumIterator;
use strum_macros::{Display, EnumIter};

/// Shared settings for an entity-owned particle emitter.
#[ecs_component]
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ParticleEmitter {
    /// The emitter settings shared by the editor and Lua.
    pub config: ParticleEmitterConfig,
}

/// Authorable settings used by the particle simulation and editor.
#[serde_as]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ParticleEmitterConfig {
    /// Determines whether particles emit in bursts or continuously.
    pub mode: ParticleEmissionMode,
    /// Selects the particle's pixel, line, or sprite appearance.
    pub visual: ParticleVisual,
    /// Sets the emitter-centered rectangular spawn area in room units.
    #[serde_as(as = "serde_with::FromInto<[f32; 2]>")]
    pub area_size: Vec2,
    /// Sets the length of line visuals in pixels.
    pub line_length: u32,
    /// Sets the shortest particle lifetime in seconds.
    pub lifetime_min: f32,
    /// Sets the longest particle lifetime in seconds.
    pub lifetime_max: f32,
    /// Sets the minimum velocity on each axis.
    #[serde_as(as = "serde_with::FromInto<[f32; 2]>")]
    pub velocity_min: Vec2,
    /// Sets the maximum velocity on each axis.
    #[serde_as(as = "serde_with::FromInto<[f32; 2]>")]
    pub velocity_max: Vec2,
    /// Sets the initial visual scale multiplier.
    pub scale_start: f32,
    /// Sets the final visual scale multiplier.
    pub scale_end: f32,
    /// Sets the particle tint color.
    pub color: Color,
    /// Sets the particle opacity at emission.
    pub opacity_start: f32,
    /// Sets the particle opacity at the end of its lifetime.
    pub opacity_end: f32,
    /// Sets the number of particles emitted per burst.
    pub burst_count: u32,
    /// Sets the continuous emission rate in particles per second.
    pub emission_rate: f32,
    /// Limits the number of live particles owned by this emitter.
    pub max_particles: usize,
    /// Seeds the emitter's deterministic particle sampling.
    pub seed: u64,
}

impl ParticleEmitterConfig {
    /// Validates the configuration and returns every field-level issue.
    pub fn validate(&self) -> Result<(), Vec<ParticleConfigIssue>> {
        let issues = ParticleConfigIssue::iter()
            .filter(|issue| issue.is_invalid_for(self))
            .collect::<Vec<_>>();

        if issues.is_empty() {
            Ok(())
        } else {
            Err(issues)
        }
    }
}

impl Default for ParticleEmitterConfig {
    fn default() -> Self {
        Self {
            mode: ParticleEmissionMode::Burst,
            visual: ParticleVisual::Pixel,
            area_size: Vec2::ZERO,
            line_length: 4,
            lifetime_min: 0.1,
            lifetime_max: 0.25,
            velocity_min: Vec2::new(-10.0, -20.0),
            velocity_max: Vec2::new(10.0, -5.0),
            scale_start: 1.0,
            scale_end: 0.0,
            color: Color::WHITE,
            opacity_start: 1.0,
            opacity_end: 0.0,
            burst_count: 8,
            emission_rate: 10.0,
            max_particles: 128,
            seed: 1,
        }
    }
}

/// Particle emission timing mode.
#[derive(Clone, Copy, Debug, Default, Display, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ParticleEmissionMode {
    /// Emit a fixed number of particles when triggered.
    #[default]
    Burst,
    /// Emit particles at a configured rate while active.
    Continuous,
}

/// Visual representation used for each particle.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ParticleVisual {
    /// Draw a one-pixel square.
    #[default]
    Pixel,
    /// Draw a short line aligned to particle velocity.
    Line,
    /// Draw a selected sprite frame.
    Sprite {
        /// Selects the texture frame and its crop metadata.
        frame: CurrentFrameSnapshot,
    },
}

/// Identifies one invalid particle configuration field.
#[derive(Clone, Copy, Debug, EnumIter, PartialEq, Eq)]
pub enum ParticleConfigIssue {
    /// The emission area must have finite, non-negative dimensions.
    AreaSize,
    /// Lifetimes must be finite, positive, and ordered.
    LifetimeRange,
    /// Velocity bounds must be finite and ordered on both axes.
    VelocityRange,
    /// Particle scales must be finite and non-negative.
    ScaleRange,
    /// Tint channels must be finite and in 0..=1.
    Color,
    /// Opacity endpoints must be in 0..=1.
    OpacityRange,
    /// Burst mode requires at least one particle.
    BurstCount,
    /// Continuous mode requires a positive finite rate.
    EmissionRate,
    /// The per-emitter limit must be positive.
    ParticleLimit,
    /// Line visuals require a positive length.
    LineLength,
    /// Sprite visuals require a valid frame.
    SpriteFrame,
}

impl ParticleConfigIssue {
    /// Returns a field-level validation message.
    pub fn message(self) -> &'static str {
        match self {
            Self::AreaSize => "Emission area dimensions must be finite and non-negative",
            Self::LifetimeRange => "Lifetime bounds must be finite, positive, and ordered",
            Self::VelocityRange => "Velocity bounds must be finite and ordered",
            Self::ScaleRange => "Scale endpoints must be finite and non-negative",
            Self::Color => "Tint channels must be finite and between zero and one",
            Self::OpacityRange => "Opacity endpoints must be between zero and one",
            Self::BurstCount => "Burst count must be positive",
            Self::EmissionRate => "Continuous emission rate must be positive and finite",
            Self::ParticleLimit => "Per-emitter particle limit must be positive",
            Self::LineLength => "Line length must be positive",
            Self::SpriteFrame => "Sprite visual requires a valid frame",
        }
    }

    fn is_invalid_for(self, config: &ParticleEmitterConfig) -> bool {
        match self {
            Self::AreaSize => {
                !is_non_negative_finite(config.area_size.x)
                    || !is_non_negative_finite(config.area_size.y)
            }
            Self::LifetimeRange => {
                !config.lifetime_min.is_finite()
                    || !config.lifetime_max.is_finite()
                    || config.lifetime_min <= 0.0
                    || config.lifetime_max < config.lifetime_min
            }
            Self::VelocityRange => {
                !config.velocity_min.x.is_finite()
                    || !config.velocity_min.y.is_finite()
                    || !config.velocity_max.x.is_finite()
                    || !config.velocity_max.y.is_finite()
                    || config.velocity_min.x > config.velocity_max.x
                    || config.velocity_min.y > config.velocity_max.y
            }
            Self::ScaleRange => {
                !is_non_negative_finite(config.scale_start)
                    || !is_non_negative_finite(config.scale_end)
            }
            Self::Color => {
                !is_unit_interval(config.color.r)
                    || !is_unit_interval(config.color.g)
                    || !is_unit_interval(config.color.b)
                    || !is_unit_interval(config.color.a)
            }
            Self::OpacityRange => {
                !is_unit_interval(config.opacity_start) || !is_unit_interval(config.opacity_end)
            }
            Self::BurstCount => {
                config.mode == ParticleEmissionMode::Burst && config.burst_count == 0
            }
            Self::EmissionRate => {
                !config.emission_rate.is_finite()
                    || (config.mode == ParticleEmissionMode::Continuous
                        && config.emission_rate <= 0.0)
            }
            Self::ParticleLimit => config.max_particles == 0,
            Self::LineLength => {
                config.visual == ParticleVisual::Line && config.line_length == 0
            }
            Self::SpriteFrame => match &config.visual {
                ParticleVisual::Sprite { frame } => {
                    !frame.has_valid_asset()
                        || !frame.frame_size.x.is_finite()
                        || !frame.frame_size.y.is_finite()
                        || frame.frame_size.x <= 0.0
                        || frame.frame_size.y <= 0.0
                        || !frame.offset.x.is_finite()
                        || !frame.offset.y.is_finite()
                }
                _ => false,
            },
        }
    }
}

fn is_non_negative_finite(value: f32) -> bool {
    value.is_finite() && value >= 0.0
}

fn is_unit_interval(value: f32) -> bool {
    value.is_finite() && (0.0..=1.0).contains(&value)
}

