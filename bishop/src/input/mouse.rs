//! Mouse button identifiers.

/// Mouse button identifiers.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

/// Distinguishes coarse wheel ticks from high-resolution pixel scrolling.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum MouseWheelKind {
    Line,
    Pixel,
}
