// engine_core/src/text/dialogue/speech_bubble.rs
use ecs_component::ecs_component;
use serde::{Deserialize, Serialize};

/// Component for displaying speech bubbles above entities.
#[ecs_component]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SpeechBubble {
    /// The text to display.
    pub text: String,
    /// Remaining time in seconds before the bubble disappears.
    pub timer: f32,
    /// Text color [r, g, b, a].
    pub color: [f32; 4],
    /// Offset from entity position (x, y).
    pub offset: (f32, f32),
    /// Font size override (uses config default if None).
    pub font_size: Option<f32>,
    /// Maximum width before word wrap (uses config default if None).
    pub max_width: Option<f32>,
    /// Whether to show a background behind the text.
    pub show_background: bool,
    /// Background color [r, g, b, a].
    pub background_color: [f32; 4],
}

impl Default for SpeechBubble {
    fn default() -> Self {
        Self {
            text: String::new(),
            timer: 3.0,
            color: [1.0, 1.0, 1.0, 1.0],
            offset: (0.0, 0.0),
            font_size: Some(2.5),
            max_width: None,
            show_background: false,
            background_color: [0.0, 0.0, 0.0, 0.7],
        }
    }
}

impl SpeechBubble {
    /// Creates a new speech bubble with the given text and duration.
    pub fn new(text: String, duration: f32) -> Self {
        Self {
            text,
            timer: duration,
            ..Default::default()
        }
    }

    /// Builder method to set the text color.
    pub fn with_color(mut self, color: [f32; 4]) -> Self {
        self.color = color;
        self
    }

    /// Builder method to set the offset.
    pub fn with_offset(mut self, x: f32, y: f32) -> Self {
        self.offset = (x, y);
        self
    }

    /// Builder method to set the font size.
    pub fn with_font_size(mut self, size: f32) -> Self {
        self.font_size = Some(size);
        self
    }

    /// Builder method to set the max width.
    pub fn with_max_width(mut self, width: f32) -> Self {
        self.max_width = Some(width);
        self
    }

    /// Builder method to set background visibility.
    pub fn with_background(mut self, show: bool) -> Self {
        self.show_background = show;
        self
    }

    /// Builder method to set the background color.
    pub fn with_background_color(mut self, color: [f32; 4]) -> Self {
        self.background_color = color;
        self
    }
}
