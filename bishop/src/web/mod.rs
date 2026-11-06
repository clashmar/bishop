//! Bishop-owned WebGL2 resources and browser rendering.

#[cfg(any(test, target_os = "emscripten"))]
mod clip;
#[cfg(any(test, target_os = "emscripten"))]
mod target;

#[cfg(target_os = "emscripten")]
mod draw;
#[cfg(target_os = "emscripten")]
mod gl_context;
#[cfg(target_os = "emscripten")]
mod impls;
#[cfg(target_os = "emscripten")]
mod primitives;
#[cfg(target_os = "emscripten")]
mod texture;
#[cfg(target_os = "emscripten")]
mod text;

#[cfg(target_os = "emscripten")]
pub use gl_context::WebContext;
#[cfg(target_os = "emscripten")]
pub use texture::WebTexture;
#[cfg(target_os = "emscripten")]
pub use target::WebRenderTarget;
