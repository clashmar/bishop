//! Render target abstractions for off-screen rendering.

#[cfg(feature = "wgpu")]
pub use crate::wgpu::{create_texture_bind_group_layout, BishopRenderTarget};

#[cfg(all(feature = "web-gl", target_os = "emscripten", not(feature = "wgpu")))]
pub use crate::web::WebRenderTarget as BishopRenderTarget;

/// Render target handle used by camera and render APIs when no concrete backend is enabled.
#[cfg(not(any(feature = "wgpu", all(feature = "web-gl", target_os = "emscripten"))))]
#[derive(Debug, Clone)]
pub struct BishopRenderTarget {
    width: u32,
    height: u32,
}

#[cfg(not(any(feature = "wgpu", all(feature = "web-gl", target_os = "emscripten"))))]
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
