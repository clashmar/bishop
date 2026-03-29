use serde::{Deserialize, Serialize};

/// Horizontal alignment for menu elements.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HorizontalAlign {
    Left,
    #[default]
    Center,
    Right,
}

/// Vertical alignment for menu elements.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VerticalAlign {
    Top,
    #[default]
    Middle,
    Bottom,
}

/// Combined alignment for menu elements.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Alignment {
    pub horizontal: HorizontalAlign,
    pub vertical: VerticalAlign,
}

impl Alignment {
    /// Creates an alignment with specified horizontal and vertical values.
    pub fn new(horizontal: HorizontalAlign, vertical: VerticalAlign) -> Self {
        Self {
            horizontal,
            vertical,
        }
    }

    /// Creates a centered alignment.
    pub fn center() -> Self {
        Self::default()
    }

    /// Creates a top-left alignment.
    pub fn top_left() -> Self {
        Self::new(HorizontalAlign::Left, VerticalAlign::Top)
    }

    /// Creates a top-center alignment.
    pub fn top_center() -> Self {
        Self::new(HorizontalAlign::Center, VerticalAlign::Top)
    }

    /// Creates a top-right alignment.
    pub fn top_right() -> Self {
        Self::new(HorizontalAlign::Right, VerticalAlign::Top)
    }
}
