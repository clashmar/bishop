use bishop::prelude::*;

/// Linearly interpolates between two positions and rounds to the nearest pixel.
#[inline]
pub fn lerp_rounded(prev_pos: Vec2, current_pos: Vec2, alpha: f32) -> Vec2 {
    (prev_pos * (1.0 - alpha) + current_pos * alpha).round()
}

/// Mitigates erratic dt by smoothing `raw_dt`, initializing from the first sample.
/// `alpha` is the weight of the previous smoothed value (higher = smoother but slower to react).
#[inline]
pub fn smooth_dt(smoothed_dt: &mut Option<f32>, raw_dt: f32, alpha: f32) -> f32 {
    let s = smoothed_dt.get_or_insert(raw_dt);
    *s = *s * alpha + raw_dt * (1.0 - alpha);
    *s
}

/// Common display refresh rates to snap frame times to (checked in order).
const SNAP_FREQUENCIES: [f32; 5] = [60.0, 120.0, 144.0, 240.0, 30.0];

/// Snaps raw_dt to the nearest common display interval if within 10% of it.
/// Eliminates accumulator drift that causes periodic stutter.
#[inline]
pub fn snap_dt(raw_dt: f32) -> f32 {
    for freq in SNAP_FREQUENCIES {
        let target = 1.0 / freq;
        if (raw_dt - target).abs() < target * 0.1 {
            return target;
        }
    }
    raw_dt
}
