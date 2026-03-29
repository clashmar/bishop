//! Text rendering and measurement.

mod dimensions;
mod params;

pub use dimensions::*;
pub use params::*;

use crate::types::Color;

/// Text rendering and measurement operations.
pub trait Text {
    /// Draws text at the specified position and returns its dimensions.
    fn draw_text(
        &mut self,
        text: &str,
        x: f32,
        y: f32,
        font_size: f32,
        color: Color,
    ) -> TextDimensions;

    /// Draws text with extended parameters including rotation support.
    fn draw_text_ex(&mut self, text: &str, x: f32, y: f32, params: TextParams) -> TextDimensions;

    /// Measures text without drawing it.
    fn measure_text(&self, text: &str, font_size: f32) -> TextDimensions;
}
