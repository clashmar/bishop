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
