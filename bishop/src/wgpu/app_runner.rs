//! Internal application runner for wgpu backend.

use std::cell::RefCell;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::Arc;

use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::window::{Fullscreen, Window, WindowId};

use super::context::WgpuContext;
use super::exec::poll_once;
use crate::window::{IconData, WindowConfig, WindowIcon};
use crate::BishopApp;

/// Internal application handler for running BishopApp with wgpu/winit.
pub(crate) struct WgpuAppRunner<A: BishopApp> {
    config: WindowConfig,
    app: Option<A>,
    ctx: Option<Rc<RefCell<WgpuContext>>>,
    window: Option<Arc<Window>>,
    initialized: bool,
    exit_requested: bool,
    init_future: Option<Pin<Box<dyn Future<Output = A>>>>,
    frame_future: Option<Pin<Box<dyn Future<Output = A>>>>,
}

impl<A: BishopApp> WgpuAppRunner<A> {
    /// Creates a new app runner with the given config and app.
    pub fn new(config: WindowConfig, app: A) -> Self {
        Self {
            config,
            app: Some(app),
            ctx: None,
            window: None,
            initialized: false,
            exit_requested: false,
            init_future: None,
            frame_future: None,
        }
    }
}

impl<A: BishopApp + 'static> ApplicationHandler for WgpuAppRunner<A> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let fullscreen = if self.config.fullscreen {
            Some(Fullscreen::Borderless(None))
        } else {
            None
        };

        let mut attrs = Window::default_attributes()
            .with_title(&self.config.title)
            .with_inner_size(LogicalSize::new(self.config.width, self.config.height))
            .with_resizable(self.config.resizable)
            .with_fullscreen(fullscreen);

        if let Some(icon) = &self.config.icon {
            if let Some(winit_icon) = convert_window_icon(icon) {
                attrs = attrs.with_window_icon(Some(winit_icon));
            }
        }

        let window = match event_loop.create_window(attrs) {
            Ok(w) => Arc::new(w),
            Err(e) => {
                eprintln!("Failed to create window: {e}");
                event_loop.exit();
                return;
            }
        };

        match WgpuContext::new_sync(window.clone()) {
            Ok(ctx) => self.ctx = Some(Rc::new(RefCell::new(ctx))),
            Err(e) => {
                eprintln!("Failed to create WgpuContext: {e}");
                event_loop.exit();
                return;
            }
        }

        self.window = Some(window);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        if let Some(ctx) = &self.ctx {
            ctx.borrow_mut().handle_window_event(&event);
        }

        match event {
            WindowEvent::CloseRequested => {
                self.exit_requested = true;
                if self.init_future.is_none() && self.frame_future.is_none() {
                    event_loop.exit();
                }
            }
            WindowEvent::RedrawRequested => {
                if let Some(ctx) = &self.ctx {
                    ctx.borrow_mut().begin_frame();

                    // Handle initialization with yielding support
                    if !self.initialized {
                        if self.init_future.is_none() {
                            let mut app = self.app.take().expect("App missing during init");
                            let ctx_clone = ctx.clone();
                            self.init_future =
                                Some(Box::pin(async move {
                                    app.init(ctx_clone).await;
                                    app
                                }));
                        }

                        if let Some(ref mut future) = self.init_future {
                            if let Some(app) = poll_once(future) {
                                self.app = Some(app);
                                self.initialized = true;
                                self.init_future = None;
                            }
                        }
                    } else {
                        // Normal frame - start new future if none pending
                        if self.frame_future.is_none() {
                            let mut app = self.app.take().expect("App missing during frame");
                            let ctx_clone = ctx.clone();
                            self.frame_future =
                                Some(Box::pin(async move {
                                    app.frame(ctx_clone).await;
                                    app
                                }));
                        }

                        if let Some(ref mut future) = self.frame_future {
                            if let Some(app) = poll_once(future) {
                                self.app = Some(app);
                                self.frame_future = None;
                                // Clear input only when frame completes
                                ctx.borrow_mut().end_frame_input();
                            }
                        }
                    }

                    if let Err(e) = ctx.borrow_mut().render_frame() {
                        eprintln!("Render error: {e}");
                    }
                    if self.exit_requested && self.init_future.is_none() && self.frame_future.is_none()
                    {
                        event_loop.exit();
                    } else if let Some(window) = &self.window {
                        window.request_redraw();
                    }
                }
            }
            _ => {}
        }
    }

    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(app) = self.app.as_mut() {
            app.on_exit();
        }
    }
}

/// Converts a WindowIcon to a winit Icon.
fn convert_window_icon(icon: &WindowIcon) -> Option<winit::window::Icon> {
    match icon {
        WindowIcon::Png(data) => {
            let img = image::load_from_memory(data).ok()?.to_rgba8();
            let (width, height) = img.dimensions();
            winit::window::Icon::from_rgba(img.into_raw(), width, height).ok()
        }
        WindowIcon::Rgba {
            small,
            medium,
            large,
        } => {
            let icon_data = large.as_ref().or(medium.as_ref()).or(small.as_ref())?;
            create_icon_from_data(icon_data)
        }
    }
}

/// Creates a winit Icon from IconData.
fn create_icon_from_data(data: &IconData) -> Option<winit::window::Icon> {
    winit::window::Icon::from_rgba(data.rgba.clone(), data.width, data.height).ok()
}
