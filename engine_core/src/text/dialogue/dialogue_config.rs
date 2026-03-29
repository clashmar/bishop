// engine_core/src/text/dialogue/dialogue_config.rs
use serde::{Deserialize, Serialize};

/// Global configuration for the dialogue system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DialogueConfig {
    /// Default duration in seconds for speech bubbles.
    pub default_duration: f32,
    /// Default font size for speech text.
    pub font_size: f32,
    /// Default maximum width before word wrap (in pixels).
    pub max_width: f32,
    /// Default vertical offset from entity position (negative = above).
    pub default_offset_y: f32,
    /// Padding around text in speech bubbles.
    pub padding: f32,
    /// Default text color [r, g, b, a].
    pub default_color: [f32; 4],
    /// Default background color [r, g, b, a].
    pub default_background_color: [f32; 4],
    /// Whether to show background by default.
    pub show_background: bool,
}

impl Default for DialogueConfig {
    fn default() -> Self {
        Self {
            default_duration: 3.0,
            font_size: 5.0,
            max_width: 50.0,
            default_offset_y: -5.0,
            padding: 2.0,
            default_color: [1.0, 1.0, 1.0, 1.0],
            default_background_color: [0.0, 0.0, 0.0, 0.7],
            show_background: false,
        }
    }
}
