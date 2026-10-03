//! Render target abstractions for off-screen rendering.

#[cfg(feature = "wgpu")]
pub use crate::wgpu::{create_texture_bind_group_layout, BishopRenderTarget};

/// Render target handle used by camera and render APIs when no concrete backend is enabled.
#[cfg(not(feature = "wgpu"))]
#[derive(Debug, Clone)]
pub struct BishopRenderTarget {
    width: u32,
    height: u32,
}

#[cfg(not(feature = "wgpu"))]
impl BishopRenderTarget {
    /// Creates a backend-neutral render target handle.
    pub fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }

    /// Returns the render target width.
    pub fn width(&self) -> u32 {
        self.width
    }

    /// Returns the render target height.
    pub fn height(&self) -> u32 {
        self.height
    }
}
