//! Text rendering and measurement.

mod dimensions;
#[cfg(any(
    all(feature = "web-gl", target_os = "emscripten"),
    all(test, any(feature = "wgpu", feature = "web-gl"))
))]
pub(crate) mod font_metrics;
mod params;

pub use dimensions::*;
pub use params::*;
pub(crate) use params::raster_scale_for_view;

use crate::types::Color;
use serde::{Deserialize, Serialize};

/// Stable identity for a registered game font.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Ord, PartialOrd, Serialize, Deserialize,
)]
pub struct FontId(pub usize);

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

    /// Measures text with the same selected font and scale parameters as `draw_text_ex`.
    fn measure_text_ex(&self, text: &str, params: &TextParams) -> TextDimensions {
        self.measure_text(text, params.font_size as f32).scaled(
            params.font_scale * params.font_scale_aspect,
            params.font_scale,
        )
    }

    /// Registers font bytes for use by `TextParams.font`.
    fn register_font(&mut self, id: FontId, ttf_data: &[u8]) -> Result<(), String>;

    /// Draws word-wrapped text within `max_width`, returning the total height used.
    fn draw_text_wrapped(
        &mut self,
        text: &str,
        x: f32,
        y: f32,
        font_size: f32,
        color: Color,
        max_width: f32,
    ) -> f32 {
        let space_w = self.measure_text(" ", font_size).width;
        let mut line = String::new();
        let mut line_w = 0.0f32;
        let mut draw_y = y;

        for word in text.split_whitespace() {
            let word_w = self.measure_text(word, font_size).width;
            if line.is_empty() {
                line.push_str(word);
                line_w = word_w;
            } else if line_w + space_w + word_w <= max_width {
                line.push(' ');
                line.push_str(word);
                line_w += space_w + word_w;
            } else {
                self.draw_text(&line, x, draw_y, font_size, color);
                draw_y += font_size;
                line = word.to_string();
                line_w = word_w;
            }
        }

        if !line.is_empty() {
            self.draw_text(&line, x, draw_y, font_size, color);
            draw_y += font_size;
        }

        draw_y - y
    }
}
