use super::{
    AREA_LABEL, BURST_COUNT_LABEL, EMISSION_RATE_LABEL, LIFETIME_LABEL, LINE_LENGTH_LABEL,
    MODE_LABEL, OPACITY_LABEL, PARTICLE_LIMIT_LABEL, SCALE_LABEL, SEED_LABEL,
    SPRITE_FRAME_LABEL, TINT_ALPHA_LABEL, TINT_LABEL, VELOCITY_X_LABEL, VELOCITY_Y_LABEL,
    VISUAL_LABEL,
};

pub(super) fn field_tooltip(label: &str) -> Option<&'static str> {
    match label {
        MODE_LABEL => Some(
            "Burst emits the configured count on request; Continuous emits at its rate while started.",
        ),
        VISUAL_LABEL => Some(
            "Choose a pixel, a line pointing in the travel direction, or a fixed sprite-frame snapshot.",
        ),
        AREA_LABEL => Some(
            "Width and height of the random spawn area around the emitter, in world pixels. Both must be at least 0.",
        ),
        LIFETIME_LABEL => Some(
            "Left is the minimum and right is the maximum lifetime in seconds. Both must be greater than 0; maximum must be at least minimum.",
        ),
        VELOCITY_X_LABEL => Some(
            "Left is minimum and right is maximum horizontal velocity, in world pixels per second. Minimum must not exceed maximum.",
        ),
        VELOCITY_Y_LABEL => Some(
            "Left is minimum and right is maximum vertical velocity, in world pixels per second. Minimum must not exceed maximum. Positive Y moves downward.",
        ),
        SCALE_LABEL => Some(
            "Left is particle scale at emission and right is scale at the end of life. Both must be at least 0. Scale multiplies pixel/sprite size and line thickness, not line length.",
        ),
        TINT_LABEL => Some(
            "RGB tint multiplied with the selected particle visual. Each channel must be finite and between 0 and 1.",
        ),
        TINT_ALPHA_LABEL => Some(
            "Base alpha multiplier; finite and between 0 and 1. It combines with the lifetime opacity setting.",
        ),
        OPACITY_LABEL => Some(
            "Left is opacity at emission and right is opacity at the end of life. Each must be finite and between 0 and 1; values interpolate over the particle lifetime.",
        ),
        LINE_LENGTH_LABEL => Some(
            "Line length in world pixels, independent of scale. Must be at least 1; scale controls line thickness.",
        ),
        SPRITE_FRAME_LABEL => Some(
            "PNG frame used by emitted particles; choose it independently of the emitter entity's sprite.",
        ),
        BURST_COUNT_LABEL => Some(
            "Particles created by each burst request. Must be at least 1 when mode is Burst.",
        ),
        EMISSION_RATE_LABEL => Some(
            "Requested particles per second while Continuous mode is started. Must be finite and greater than 0 in Continuous mode.",
        ),
        PARTICLE_LIMIT_LABEL => Some(
            "Maximum live particles for this emitter; must be at least 1. The shared pool caps all emitters at 4096 live particles.",
        ),
        SEED_LABEL => Some(
            "Base seed (0 through 2^64-1) for this emitter's deterministic random stream. Successive bursts vary; replaying the same emission sequence with the same seed reproduces it.",
        ),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_particle_setting_label_has_a_tooltip() {
        let labels = [
            MODE_LABEL,
            VISUAL_LABEL,
            AREA_LABEL,
            LIFETIME_LABEL,
            VELOCITY_X_LABEL,
            VELOCITY_Y_LABEL,
            SCALE_LABEL,
            TINT_LABEL,
            TINT_ALPHA_LABEL,
            OPACITY_LABEL,
            LINE_LENGTH_LABEL,
            SPRITE_FRAME_LABEL,
            BURST_COUNT_LABEL,
            EMISSION_RATE_LABEL,
            PARTICLE_LIMIT_LABEL,
            SEED_LABEL,
        ];

        for label in labels {
            assert!(field_tooltip(label).is_some(), "missing tooltip for {label}");
        }
    }

    #[test]
    fn range_tooltips_explain_endpoint_order_and_validation_bounds() {
        let lifetime = field_tooltip(LIFETIME_LABEL).unwrap();
        assert!(lifetime.contains("Left is the minimum and right is the maximum"));
        assert!(lifetime.contains("greater than 0"));
        assert!(lifetime.contains("at least minimum"));

        let velocity_x = field_tooltip(VELOCITY_X_LABEL).unwrap();
        assert!(velocity_x.contains("minimum and right is maximum"));
        assert!(velocity_x.contains("must not exceed maximum"));

        let scale = field_tooltip(SCALE_LABEL).unwrap();
        assert!(scale.contains("at least 0"));

        let opacity = field_tooltip(OPACITY_LABEL).unwrap();
        assert!(opacity.contains("between 0 and 1"));
    }
}
