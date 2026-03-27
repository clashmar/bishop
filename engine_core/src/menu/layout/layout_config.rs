use super::{Alignment, LayoutDirection, Padding};
use serde::{Deserialize, Serialize};

/// Configuration for menu element layout.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct LayoutConfig {
    pub direction: LayoutDirection,
    pub spacing: f32,
    pub padding: Padding,
    pub alignment: Alignment,
    pub item_width: f32,
    pub item_height: f32,
}

impl Default for LayoutConfig {
    fn default() -> Self {
        Self {
            direction: LayoutDirection::Vertical,
            spacing: 16.0,
            padding: Padding::uniform(32.0),
            alignment: Alignment::center(),
            item_width: 200.0,
            item_height: 40.0,
        }
    }
}

impl LayoutConfig {
    /// Creates a new layout configuration with specified direction.
    pub fn new(direction: LayoutDirection) -> Self {
        Self {
            direction,
            ..Default::default()
        }
    }

    /// Creates a vertical layout.
    pub fn vertical() -> Self {
        Self::new(LayoutDirection::Vertical)
    }

    /// Creates a horizontal layout.
    pub fn horizontal() -> Self {
        Self::new(LayoutDirection::Horizontal)
    }

    /// Creates a grid layout with specified columns.
    pub fn grid(columns: u32) -> Self {
        Self::new(LayoutDirection::Grid { columns })
    }

    /// Sets the spacing between elements.
    pub fn with_spacing(mut self, spacing: f32) -> Self {
        self.spacing = spacing;
        self
    }

    /// Sets the padding around the layout.
    pub fn with_padding(mut self, padding: Padding) -> Self {
        self.padding = padding;
        self
    }

    /// Sets the alignment for elements.
    pub fn with_alignment(mut self, alignment: Alignment) -> Self {
        self.alignment = alignment;
        self
    }

    /// Sets the default item size.
    pub fn with_item_size(mut self, width: f32, height: f32) -> Self {
        self.item_width = width;
        self.item_height = height;
        self
    }
}
