//! Texture loading capability.

use crate::types::Texture2D;

/// Synchronous texture loading capability provided by the graphics backend.
pub trait TextureLoader {
    /// Load a texture from raw PNG bytes.
    fn load_texture_from_bytes(&self, data: &[u8]) -> Result<Texture2D, String>;
    /// Load a texture from an absolute file path.
    fn load_texture_from_path(&self, path: &str) -> Result<Texture2D, String>;
    /// Create a 1×1 transparent placeholder texture.
    fn empty_texture(&self) -> Texture2D;
}
