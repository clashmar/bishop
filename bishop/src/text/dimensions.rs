//! Text measurement dimensions.

/// Dimensions returned from text measurement and rendering.
#[derive(Clone, Copy, Debug, Default)]
pub struct TextDimensions {
    /// Width of the text in pixels.
    pub width: f32,
    /// Height of the text in pixels.
    pub height: f32,
    /// Vertical offset from baseline.
    pub offset_y: f32,
}

impl TextDimensions {
    /// Scales text dimensions horizontally and vertically.
    pub fn scaled(self, horizontal_scale: f32, vertical_scale: f32) -> Self {
        Self {
            width: self.width * horizontal_scale,
            height: self.height * vertical_scale,
            offset_y: self.offset_y * vertical_scale,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::TextDimensions;

    #[test]
    fn scaled_applies_independent_horizontal_and_vertical_factors() {
        let dimensions = TextDimensions {
            width: 12.0,
            height: 8.0,
            offset_y: 6.0,
        };
        let scaled = dimensions.scaled(2.0, 0.5);

        assert_eq!(scaled.width, 24.0);
        assert_eq!(scaled.height, 4.0);
        assert_eq!(scaled.offset_y, 3.0);
    }
}
