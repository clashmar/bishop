//! Core types for the bishop graphics abstraction.

mod color;
mod rect;

pub use color::*;
pub use rect::*;

pub use glam::{ivec2, vec2, vec3, IVec2, Mat2, Mat4, Vec2, Vec3};

/// Texture handle used by drawing APIs.
#[derive(Clone)]
pub struct Texture2D {
    width: u32,
    height: u32,
    #[cfg(feature = "wgpu")]
    texture: std::sync::Arc<crate::wgpu::WgpuTexture>,
}

impl Texture2D {
    /// Creates a new Texture2D from a WgpuTexture.
    #[cfg(feature = "wgpu")]
    pub fn from_wgpu(texture: crate::wgpu::WgpuTexture) -> Self {
        let width = texture.width();
        let height = texture.height();
        Self {
            width,
            height,
            texture: std::sync::Arc::new(texture),
        }
    }

    /// Creates a new Texture2D from an Arc-wrapped WgpuTexture.
    #[cfg(feature = "wgpu")]
    pub fn from_wgpu_arc(texture: std::sync::Arc<crate::wgpu::WgpuTexture>) -> Self {
        Self {
            width: texture.width(),
            height: texture.height(),
            texture,
        }
    }

    /// Returns a reference to the underlying WgpuTexture.
    #[cfg(feature = "wgpu")]
    pub fn inner(&self) -> &crate::wgpu::WgpuTexture {
        &self.texture
    }

    /// Returns the Arc-wrapped WgpuTexture.
    #[cfg(feature = "wgpu")]
    pub fn inner_arc(&self) -> &std::sync::Arc<crate::wgpu::WgpuTexture> {
        &self.texture
    }

    /// Returns the texture width in pixels.
    pub fn width(&self) -> f32 {
        self.width as f32
    }

    /// Returns the texture height in pixels.
    pub fn height(&self) -> f32 {
        self.height as f32
    }
}
