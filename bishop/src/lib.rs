//! Bishop - Backend abstraction traits for the bishop-engine.
//!
//! This crate provides trait abstractions for input, drawing, and text rendering
//! that can be implemented by different backends (winit+wgpu, console, etc.).
//!
//! # Backend Support
//!
//! The `BishopContext` trait can be implemented for any backend:
//! - Graphics backends (wgpu) implement full rendering
//! - Console backends can implement with text-based or stub graphics
//! - Headless backends can implement with no-op rendering for testing
//!
//! # Example
//!
//! ```ignore
//! use bishop::prelude::*;
//!
//! fn draw_button<C: BishopContext>(ctx: &mut C, rect: Rect, label: &str) -> bool {
//!     let mouse = ctx.mouse_position();
//!     let hovered = rect.contains(Vec2::new(mouse.0, mouse.1));
//!
//!     let bg_color = if hovered { Color::GRAY } else { Color::BLACK };
//!     ctx.draw_rectangle(rect.x, rect.y, rect.w, rect.h, bg_color);
//!     ctx.draw_text(label, rect.x + 5.0, rect.y + 20.0, 16.0, Color::WHITE);
//!
//!     ctx.is_mouse_button_pressed(MouseButton::Left) && hovered
//! }
//! ```

pub mod camera;
pub mod draw;
pub mod input;
pub mod material;
pub mod text;
pub mod texture;
pub mod time;
pub mod types;
pub mod window;

#[cfg(feature = "wgpu")]
pub mod wgpu;

#[cfg(feature = "audio")]
pub mod audio;

pub use camera::*;
pub use draw::*;
pub use input::*;
pub use text::*;
pub use texture::*;
pub use time::*;
pub use types::*;
pub use window::*;

use std::cell::RefCell;
use std::rc::Rc;

#[cfg(feature = "wgpu")]
pub use wgpu::WgpuContext;

use material::RenderOps;

/// Combined context trait for widgets that need input, drawing, text, camera, window, time, render operations, and texture loading.
pub trait BishopContext:
    Input + Draw + Text + Camera + Window + Time + RenderOps + TextureLoader
{
}

impl<T: Input + Draw + Text + Camera + Window + Time + RenderOps + TextureLoader> BishopContext
    for T
{
}

/// Trait for applications that can be run by bishop.
pub trait BishopApp {
    /// Called once after the backend is ready but before the main loop.
    /// Use for async initialization (loading assets, setting up state, etc.).
    /// Default implementation is a no-op.
    fn init(&mut self, _ctx: PlatformContext) -> impl std::future::Future<Output = ()> {
        async {}
    }

    /// Called once per frame. The app handles its own update/render logic.
    fn frame(&mut self, ctx: PlatformContext) -> impl std::future::Future<Output = ()>;

    /// Called when the application is about to exit. Default is a no-op.
    fn on_exit(&mut self) {}
}

/// Error type for the wgpu run function.
#[cfg(feature = "wgpu")]
#[derive(Debug)]
pub enum RunError {
    /// Event loop creation or execution failed.
    EventLoop(String),
    /// Graphics/wgpu initialization failed.
    Graphics(String),
    /// Window creation failed.
    Window(String),
}

#[cfg(feature = "wgpu")]
impl std::fmt::Display for RunError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RunError::EventLoop(msg) => write!(f, "Event loop error: {msg}"),
            RunError::Graphics(msg) => write!(f, "Graphics error: {msg}"),
            RunError::Window(msg) => write!(f, "Window error: {msg}"),
        }
    }
}

#[cfg(feature = "wgpu")]
impl std::error::Error for RunError {}

/// Runs a BishopApp with the wgpu backend.
#[cfg(feature = "wgpu")]
pub fn run_wgpu<A: BishopApp + 'static>(
    config: window::WindowConfig,
    app: A,
) -> Result<(), RunError> {
    use wgpu::app_runner::WgpuAppRunner;
    use winit::event_loop::EventLoop;

    let event_loop = EventLoop::new().map_err(|e| RunError::EventLoop(e.to_string()))?;
    let mut runner = WgpuAppRunner::new(config, app);
    event_loop
        .run_app(&mut runner)
        .map_err(|e| RunError::EventLoop(e.to_string()))?;
    Ok(())
}

/// Runs a BishopApp with the appropriate backend for the current platform.
///
/// This is the main entry point for running bishop applications. It automatically
/// selects and runs the correct backend based on enabled features:
/// - `wgpu`: Uses the native wgpu/winit backend (desktop platforms)
///
/// # Example
///
/// ```ignore
/// use bishop::prelude::*;
///
/// struct MyApp;
///
/// impl BishopApp for MyApp {
///     async fn frame(&mut self, ctx: &mut impl BishopContext) {
///         // Game logic here
///     }
/// }
///
/// fn main() -> Result<(), RunError> {
///     let config = WindowConfig::new("My Game").with_size(800, 600);
///     run_backend(config, MyApp)
/// }
/// ```
#[cfg(feature = "wgpu")]
pub fn run_backend<A: BishopApp + 'static>(
    config: window::WindowConfig,
    app: A,
) -> Result<(), RunError> {
    run_wgpu(config, app)
}

/// The context type for the active graphics backend.
///
/// This is a type alias that resolves to:
/// - `Rc<RefCell<WgpuContext>>` when the `wgpu` feature is enabled
///
/// Use this at application entry points (main.rs) to create the context.
/// For function parameters, prefer `impl BishopContext` for flexibility.
#[cfg(feature = "wgpu")]
pub type PlatformContext = Rc<RefCell<wgpu::WgpuContext>>;

/// Prelude module for convenient glob imports.
///
/// # Example
///
/// ```ignore
/// use bishop::prelude::*;
/// ```
pub mod prelude {
    pub use crate::camera::*;
    pub use crate::draw::*;
    pub use crate::input::*;
    pub use crate::material::*;
    pub use crate::text::*;
    pub use crate::texture::*;
    pub use crate::time::*;
    pub use crate::types::*;
    pub use crate::window::*;
    pub use crate::BishopApp;
    pub use crate::BishopContext;
    pub use glam::{vec4, Vec2, Vec3};

    #[cfg(feature = "wgpu")]
    pub use crate::wgpu::WgpuContext;

    #[cfg(feature = "wgpu")]
    pub use crate::{run_backend, run_wgpu, PlatformContext, RunError};

    #[cfg(feature = "audio")]
    pub use crate::audio::AudioBackend;

    #[cfg(feature = "audio-cpal")]
    pub use crate::audio::PlatformAudioBackend;
}
