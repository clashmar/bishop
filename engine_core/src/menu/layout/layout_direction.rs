use serde::{Deserialize, Serialize};

/// Direction for menu element layout.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LayoutDirection {
    /// Stack elements vertically.
    #[default]
    Vertical,
    /// Stack elements horizontally.
    Horizontal,
    /// Arrange elements in a grid with specified columns.
    Grid { columns: u32 },
}
