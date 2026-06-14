//! Text rendering parameters.

use crate::types::Color;

/// Arguments for "draw_text_ex" function such as font, font_size etc
#[derive(Debug, Clone)]
pub struct TextParams {
    pub font: Option<()>,
    /// Base size for character height. The size in pixel used during font rasterizing.
    pub font_size: u16,
    /// The glyphs sizes actually drawn on the screen will be font_size * font_scale
    /// However with font_scale too different from 1.0 letters may be blurry
    pub font_scale: f32,
    /// Font X axis would be scaled by font_scale * font_scale_aspect
    /// and Y axis would be scaled by font_scale
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
