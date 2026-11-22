use super::*;
use bishop::types::{Color, Vec2};
use ron::{
    from_str,
    ser::{to_string_pretty, PrettyConfig},
};

#[test]
fn emission_mode_display_uses_editor_labels() {
    assert_eq!(ParticleEmissionMode::Burst.to_string(), "Burst");
    assert_eq!(ParticleEmissionMode::Continuous.to_string(), "Continuous");
}

#[test]
fn validate_accepts_default_config() {
    assert!(ParticleEmitterConfig::default().validate().is_ok());
}

#[test]
fn validate_rejects_zero_lifetime() {
    let config = ParticleEmitterConfig {
        lifetime_min: 0.0,
        ..ParticleEmitterConfig::default()
    };

    assert!(config.validate().is_err());
}

#[test]
fn validate_accepts_upward_velocity_range_from_negative_to_zero() {
    let config = ParticleEmitterConfig {
        velocity_min: Vec2::new(0.0, -12.0),
        velocity_max: Vec2::new(0.0, 0.0),
        ..ParticleEmitterConfig::default()
    };

    assert!(config.validate().is_ok());
}

#[test]
fn validate_rejects_reversed_velocity_range() {
    let config = ParticleEmitterConfig {
        velocity_min: Vec2::new(2.0, 0.0),
        velocity_max: Vec2::new(1.0, 1.0),
        ..ParticleEmitterConfig::default()
    };

    assert!(config.validate().is_err());
}

#[test]
fn validate_rejects_opacity_above_one() {
    let config = ParticleEmitterConfig {
        opacity_end: 1.01,
        ..ParticleEmitterConfig::default()
    };

    assert!(config.validate().is_err());
}

#[test]
fn validate_rejects_negative_emission_area() {
    let config = ParticleEmitterConfig {
        area_size: Vec2::new(-1.0, 0.0),
        ..ParticleEmitterConfig::default()
    };

    assert!(config.validate().is_err());
}

#[test]
fn validate_rejects_reversed_lifetime_range() {
    let config = ParticleEmitterConfig {
        lifetime_min: 2.0,
        lifetime_max: 1.0,
        ..ParticleEmitterConfig::default()
    };

    assert!(config.validate().is_err());
}

#[test]
fn validate_rejects_negative_scale() {
    let config = ParticleEmitterConfig {
        scale_end: -0.1,
        ..ParticleEmitterConfig::default()
    };

    assert!(config.validate().is_err());
}

#[test]
fn validate_rejects_zero_particle_limit() {
    let config = ParticleEmitterConfig {
        max_particles: 0,
        ..ParticleEmitterConfig::default()
    };

    assert!(config.validate().is_err());
}

#[test]
fn validate_rejects_non_finite_tint() {
    let config = ParticleEmitterConfig {
        color: Color::new(f32::NAN, 1.0, 1.0, 1.0),
        ..ParticleEmitterConfig::default()
    };

    assert!(config.validate().is_err());
}

#[test]
fn validate_rejects_zero_burst_count() {
    let config = ParticleEmitterConfig {
        burst_count: 0,
        ..ParticleEmitterConfig::default()
    };

    assert!(config.validate().is_err());
}

#[test]
fn validate_rejects_non_finite_rate_in_burst_mode() {
    let config = ParticleEmitterConfig {
        emission_rate: f32::NAN,
        ..ParticleEmitterConfig::default()
    };

    assert!(config.validate().is_err());
}

#[test]
fn validate_rejects_zero_rate_in_continuous_mode() {
    let config = ParticleEmitterConfig {
        mode: ParticleEmissionMode::Continuous,
        emission_rate: 0.0,
        ..ParticleEmitterConfig::default()
    };

    assert!(config.validate().is_err());
}

#[test]
fn ron_round_trip_preserves_configuration() {
    let config = ParticleEmitterConfig::default();
    let serialized = match to_string_pretty(&config, PrettyConfig::new()) {
        Ok(serialized) => serialized,
        Err(error) => panic!("particle config serialization failed: {error}"),
    };
    let deserialized = match from_str::<ParticleEmitterConfig>(&serialized) {
        Ok(deserialized) => deserialized,
        Err(error) => panic!("particle config deserialization failed: {error}"),
    };

    assert_eq!(deserialized, config);
}
