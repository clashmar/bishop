use super::{ParticleEmitterModule, apply_config_candidate, validation_messages};
use engine_core::ecs::inspector::module::InspectorModule;
use bishop::types::Vec2;
use engine_core::animation::ClipId;
use engine_core::ecs::components::{
    CurrentFrameSnapshot, ParticleConfigIssue, ParticleEmissionMode, ParticleEmitter,
    ParticleEmitterConfig, ParticleVisual, SpriteId,
};
use engine_core::ecs::inspector::factory::MODULES;

#[test]
fn particle_emitter_has_a_registered_inspector() {
    assert!(MODULES
        .iter()
        .any(|entry| entry.type_name == ParticleEmitter::TYPE_NAME));
}

#[test]
fn mode_specific_controls_follow_emitter_mode_and_visual() {
    let mut config = ParticleEmitterConfig::default();

    let burst_pixel_controls = ParticleEmitterModule::visible_controls(&config);
    assert!(burst_pixel_controls.burst_count);
    assert!(!burst_pixel_controls.emission_rate);
    assert!(!burst_pixel_controls.line_length);
    assert!(!burst_pixel_controls.sprite_frame);

    config.mode = ParticleEmissionMode::Continuous;
    config.visual = ParticleVisual::Line;
    let continuous_line_controls = ParticleEmitterModule::visible_controls(&config);
    assert!(!continuous_line_controls.burst_count);
    assert!(continuous_line_controls.emission_rate);
    assert!(continuous_line_controls.line_length);
    assert!(!continuous_line_controls.sprite_frame);

    config.visual = ParticleVisual::Sprite {
        frame: CurrentFrameSnapshot {
            clip_id: ClipId::Idle,
            col: 0,
            row: 0,
            offset: Vec2::ZERO,
            sprite_id: SpriteId(1),
            frame_size: Vec2::new(16.0, 16.0),
            flip_x: false,
        },
    };
    let sprite_controls = ParticleEmitterModule::visible_controls(&config);
    assert!(!sprite_controls.line_length);
    assert!(sprite_controls.sprite_frame);
}

#[test]
fn setting_labels_are_short_enough_to_preserve_field_width() {
    use super::config_fields::{
        LIFETIME_LABEL, OPACITY_LABEL, SCALE_LABEL, VELOCITY_X_LABEL, VELOCITY_Y_LABEL,
    };

    assert_eq!(LIFETIME_LABEL, "Lifetime:");
    assert_eq!(VELOCITY_X_LABEL, "X velocity:");
    assert_eq!(VELOCITY_Y_LABEL, "Y velocity:");
    assert_eq!(SCALE_LABEL, "Scale:");
    assert_eq!(OPACITY_LABEL, "Opacity:");
}

#[test]
fn validation_message_height_is_included_in_body_layout() {
    let module = ParticleEmitterModule::default();
    let base_height = module.body_layout().height();
    module.validation_height.set(48.0);

    assert_eq!(module.body_layout().height(), base_height + super::FIELD_GAP + 48.0);
}

#[test]
fn valid_config_candidate_replaces_emitter_settings() {
    let mut emitter = ParticleEmitter::default();
    let mut candidate = emitter.config.clone();
    candidate.scale_start = 1.75;

    apply_config_candidate(&mut emitter, candidate).expect("valid settings should be applied");

    assert_eq!(emitter.config.scale_start, 1.75);
}

#[test]
fn valid_picker_alpha_candidate_updates_emitter_color() {
    let mut emitter = ParticleEmitter::default();
    let mut candidate = emitter.config.clone();
    candidate.color.a = 0.35;

    apply_config_candidate(&mut emitter, candidate).expect("valid alpha should be applied");

    assert_eq!(emitter.config.color.a, 0.35);
}

#[test]
fn invalid_config_candidate_preserves_settings_and_returns_field_feedback() {
    let mut emitter = ParticleEmitter::default();
    let original = emitter.config.clone();
    let mut candidate = original.clone();
    candidate.opacity_start = 1.5;

    let issues = apply_config_candidate(&mut emitter, candidate)
        .expect_err("opacity outside the unit interval should be rejected");

    assert_eq!(emitter.config, original);
    assert_eq!(issues, vec![ParticleConfigIssue::OpacityRange]);
    assert_eq!(
        validation_messages(&issues),
        vec![ParticleConfigIssue::OpacityRange.message()]
    );
}
