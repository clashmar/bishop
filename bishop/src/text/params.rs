//! Text rendering parameters.

use crate::text::FontId;
use crate::types::Color;

/// Arguments for "draw_text_ex" function such as font, font_size etc
#[derive(Debug, Clone)]
pub struct TextParams {
    /// Registered game font to use; `None` selects Bishop's embedded default font.
    pub font: Option<FontId>,
    /// Base character height in logical pixels.
    pub font_size: u16,
    /// Scales the requested logical font size before drawing and rasterization.
    pub font_scale: f32,
    /// Additional horizontal scale applied after rasterization
    /// Default is 1.0
    pub font_scale_aspect: f32,
    /// Text rotation in radians
    /// Default is 0.0
    pub rotation: f32,
    pub color: Color,
}

impl Default for TextParams {
    fn default() -> TextParams {
        TextParams {
            font: None,
            font_size: 20,
            font_scale: 1.0,
            font_scale_aspect: 1.0,
            color: Color::WHITE,
            rotation: 0.0,
        }
    }
}

pub(crate) fn raster_scale_for_view(display_scale: f32, zoom_x: f32, zoom_y: f32) -> f32 {
    (display_scale.max(0.01) * zoom_x.abs().max(zoom_y.abs()).max(0.01)).max(0.01)
}

pub(crate) fn raster_scale_for_camera(
    display_scale: f32,
    viewport_width: f32,
    viewport_height: f32,
    zoom_x: f32,
    zoom_y: f32,
) -> f32 {
    raster_scale_for_view(
        display_scale,
        viewport_width * zoom_x.abs() * 0.5,
        viewport_height * zoom_y.abs() * 0.5,
    )
}

/// The canonical set of font sizes used for atlas-efficient text rendering.
pub const FONT_SIZES: &[f32] = &[
    10.0, 12.0, 14.0, 16.0, 18.0, 20.0, 24.0, 28.0, 32.0, 36.0, 48.0, 64.0, 96.0, 128.0,
    192.0,
];

/// Snaps a font size to the nearest value in `FONT_SIZES`.
pub fn snap_font_size(raw: f32) -> f32 {
    FONT_SIZES
        .iter()
        .copied()
        .min_by(|a, b| (a - raw).abs().partial_cmp(&(b - raw).abs()).unwrap_or(std::cmp::Ordering::Equal))
        .unwrap_or(raw)
}

#[cfg(test)]
mod tests {
    use super::{raster_scale_for_camera, raster_scale_for_view};

    #[test]
    fn raster_scale_includes_display_scale_and_max_camera_zoom() {
        assert_eq!(raster_scale_for_view(2.0, 1.5, 2.0), 4.0);
    }

    #[test]
    fn raster_scale_converts_normalized_camera_zoom_to_pixels_per_world_unit() {
        assert_eq!(
            raster_scale_for_camera(1.0, 800.0, 600.0, 2.0 / 800.0, 2.0 / 600.0),
            1.0,
        );
        assert_eq!(
            raster_scale_for_camera(2.0, 800.0, 600.0, 2.0 / 800.0, 2.0 / 600.0),
            2.0,
        );
    }
}
