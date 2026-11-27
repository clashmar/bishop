//! Material and shader abstractions for GPU rendering.

pub use crate::wgpu::{
    AmbientMaterial, FinalCompositeMaterial, FullscreenQuadRenderer,
    SceneCompositeMaterial, SpotMaterial, UndarkenedMaterial,
};

pub use crate::wgpu::{
    AmbientUniforms, ModelUniforms, SpotLightData, SpotUniforms,
};
