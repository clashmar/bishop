//! Wgpu backend for bishop.
//!
//! This module provides a wgpu-based implementation of the bishop traits,
//! using winit for window management and input handling.

pub(crate) mod app_runner;
mod context;
mod conversions;
mod exec;
mod impls;
mod render;
mod state;
pub use context::WgpuContext;
pub use exec::FrameFuture;
pub use render::*;
pub use state::GraphicsStateError;
