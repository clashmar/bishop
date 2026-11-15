//! State management modules for wgpu backend.

mod graphics;
mod time;

pub use graphics::{GraphicsState, GraphicsStateError};
pub use time::TimeState;
