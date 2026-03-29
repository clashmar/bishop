//! RGBA color type with common color constants.

use serde::{Deserialize, Serialize};

/// RGBA color with components in 0.0-1.0 range.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    /// Creates a new color from RGBA components.
    pub const fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    /// Creates a color from RGB components with full opacity.
    pub const fn rgb(r: f32, g: f32, b: f32) -> Self {
        Self { r, g, b, a: 1.0 }
    }

    pub const LIGHTGREY: Color = Color::new(0.78, 0.78, 0.78, 1.00);
    pub const GREY: Color = Color::new(0.51, 0.51, 0.51, 1.00);
    pub const DARKGRAY: Color = Color::new(0.31, 0.31, 0.31, 1.00);
    pub const YELLOW: Color = Color::new(0.99, 0.98, 0.00, 1.00);
    pub const GOLD: Color = Color::new(1.00, 0.80, 0.00, 1.00);
    pub const ORANGE: Color = Color::new(1.00, 0.63, 0.00, 1.00);
    pub const PINK: Color = Color::new(1.00, 0.43, 0.76, 1.00);
    pub const RED: Color = Color::new(0.90, 0.16, 0.22, 1.00);
    pub const MAROON: Color = Color::new(0.75, 0.13, 0.22, 1.00);
    pub const GREEN: Color = Color::new(0.00, 0.89, 0.19, 1.00);
    pub const LIME: Color = Color::new(0.00, 0.62, 0.18, 1.00);
    pub const DARKGREEN: Color = Color::new(0.00, 0.46, 0.17, 1.00);
    pub const SKYBLUE: Color = Color::new(0.40, 0.75, 1.00, 1.00);
    pub const BLUE: Color = Color::new(0.00, 0.47, 0.95, 1.00);
    pub const DARKBLUE: Color = Color::new(0.00, 0.32, 0.67, 1.00);
    pub const PURPLE: Color = Color::new(0.78, 0.48, 1.00, 1.00);
    pub const VIOLET: Color = Color::new(0.53, 0.24, 0.75, 1.00);
    pub const DARKPURPLE: Color = Color::new(0.44, 0.12, 0.49, 1.00);
    pub const BEIGE: Color = Color::new(0.83, 0.69, 0.51, 1.00);
    pub const BROWN: Color = Color::new(0.50, 0.42, 0.31, 1.00);
    pub const DARKBROWN: Color = Color::new(0.30, 0.25, 0.18, 1.00);
    pub const WHITE: Color = Color::new(1.00, 1.00, 1.00, 1.00);
    pub const BLACK: Color = Color::new(0.00, 0.00, 0.00, 1.00);
    pub const MAGENTA: Color = Color::new(1.00, 0.00, 1.00, 1.00);
    pub const TRANSPARENT: Color = Color::new(0.00, 0.00, 0.00, 0.00);

    /// Parses a 6-character hex RGB string into a Color with alpha 1.0.
    /// Accepts with or without leading `#`. Returns `None` on invalid input.
    pub fn from_hex(hex: &str) -> Option<Color> {
        let hex = hex.strip_prefix('#').unwrap_or(hex);
        if hex.len() != 6 {
            return None;
        }
        let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
        let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
        let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
        Some(Color::new(
            r as f32 / 255.0,
            g as f32 / 255.0,
            b as f32 / 255.0,
            1.0,
        ))
    }

    /// Returns the color as a 6-character uppercase hex string (no `#` prefix).
    pub fn to_hex(&self) -> String {
        let r = (self.r * 255.0).round() as u8;
        let g = (self.g * 255.0).round() as u8;
        let b = (self.b * 255.0).round() as u8;
        format!("{:02X}{:02X}{:02X}", r, g, b)
    }
}

impl From<[f32; 4]> for Color {
    fn from(arr: [f32; 4]) -> Self {
        Self {
            r: arr[0],
            g: arr[1],
            b: arr[2],
            a: arr[3],
        }
    }
}

impl From<Color> for [f32; 4] {
    fn from(c: Color) -> Self {
        [c.r, c.g, c.b, c.a]
    }
}
