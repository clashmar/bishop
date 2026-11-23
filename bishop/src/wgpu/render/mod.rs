//! Rendering primitives for the wgpu backend.

mod fullscreen_quad;
mod lighting;
mod material;
mod primitive;
mod render_target;
mod sampler;
mod text;
mod texture;
mod uniforms;
mod vertex;

pub use fullscreen_quad::FullscreenQuadRenderer;
pub(crate) use lighting::LightingRenderer;
pub use material::{
    AmbientMaterial, FinalCompositeMaterial, GlowMaterial, GridMaterial, SceneCompositeMaterial,
    SpotMaterial, UndarkenedMaterial,
};
pub use primitive::PrimitiveRenderer;
pub use render_target::{create_texture_bind_group_layout, BishopRenderTarget};
pub use text::{FontAtlas, TextRenderer};
pub use texture::{TextureRenderer, WgpuTexture};
pub use uniforms::{
    AmbientUniforms, CameraUniforms, GlowData, GlowUniforms, GridUniforms, ModelUniforms,
    SpotLightData, SpotUniforms,
};
