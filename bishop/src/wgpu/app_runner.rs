//! Internal application runner for wgpu backend.

use std::cell::RefCell;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::Arc;

use image::ImageEncoder;
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
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

    fn take_app_or_exit(&mut self, event_loop: &ActiveEventLoop, stage: &'static str) -> Option<A> {
        debug_assert!(
            self.app.is_some(),
            "runner invariant: app missing before {stage} future creation"
        );

        let Some(app) = self.app.take() else {
            eprintln!(
                "WgpuAppRunner invariant violated: app missing before {stage} future creation"
            );
            event_loop.exit();
            return None;
        };

        Some(app)
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

        if let Some(icon) = self.config.resolve_window_icon() {
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

        apply_native_icons(&window, &self.config);

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
                if let Some(ctx) = self.ctx.clone() {
                    ctx.borrow_mut().begin_frame();

                    // Handle initialization with yielding support
                    if !self.initialized {
                        if self.init_future.is_none() {
                            let Some(mut app) = self.take_app_or_exit(event_loop, "init") else {
                                return;
                            };
                            let ctx_clone = ctx.clone();
                            self.init_future = Some(Box::pin(async move {
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
                            let Some(mut app) = self.take_app_or_exit(event_loop, "frame") else {
                                return;
                            };
                            let ctx_clone = ctx.clone();
                            self.frame_future = Some(Box::pin(async move {
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
                    if self.exit_requested
                        && self.init_future.is_none()
                        && self.frame_future.is_none()
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

fn icon_data_for_window(icon: &WindowIcon) -> Option<&IconData> {
    match icon {
        WindowIcon::Png(_) => None,
        WindowIcon::Rgba {
            small,
            medium,
            large,
        } => small.as_ref().or(medium.as_ref()).or(large.as_ref()),
    }
}

fn icon_data_for_app(icon: &WindowIcon) -> Option<&IconData> {
    match icon {
        WindowIcon::Png(_) => None,
        WindowIcon::Rgba {
            small,
            medium,
            large,
        } => large.as_ref().or(medium.as_ref()).or(small.as_ref()),
    }
}

fn icon_png_bytes(icon: &WindowIcon, prefer_large: bool) -> Option<Vec<u8>> {
    match icon {
        WindowIcon::Png(data) => Some(data.clone()),
        WindowIcon::Rgba { .. } => {
            let data = if prefer_large {
                icon_data_for_app(icon)
            } else {
                icon_data_for_window(icon)
            }?;
            let image =
                image::RgbaImage::from_raw(data.width, data.height, data.rgba.clone())?;
            let mut png_bytes = Vec::new();
            image::codecs::png::PngEncoder::new(&mut png_bytes)
                .write_image(
                    image.as_raw(),
                    data.width,
                    data.height,
                    image::ExtendedColorType::Rgba8,
                )
                .ok()?;
            Some(png_bytes)
        }
    }
}

fn apply_native_icons(window: &Arc<Window>, config: &WindowConfig) {
    #[cfg(target_os = "windows")]
    apply_windows_icons(window, config);

    #[cfg(target_os = "macos")]
    apply_macos_icons(window, config);
}

#[cfg(target_os = "windows")]
fn apply_windows_icons(window: &Arc<Window>, config: &WindowConfig) {
    use windows_sys::Win32::Foundation::{HICON, HWND, LPARAM};
    use windows_sys::Win32::Graphics::Gdi::{
        CreateBitmap, CreateDIBSection, DeleteObject, GetDC, ReleaseDC, BI_BITFIELDS,
        BITMAPINFO, BITMAPV5HEADER, DIB_RGB_COLORS,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        CreateIconIndirect, GetSystemMetrics, SendMessageW, ICONINFO, ICON_BIG, ICON_SMALL,
        SM_CXICON, SM_CXSMICON, SM_CYICON, SM_CYSMICON, WM_SETICON,
    };

    let Ok(handle) = window.window_handle() else {
        return;
    };
    let RawWindowHandle::Win32(handle) = handle.as_raw() else {
        return;
    };
    let hwnd = handle.hwnd.get() as HWND;

    let small_source = config
        .resolve_window_icon()
        .and_then(icon_data_for_window)
        .or_else(|| config.resolve_app_icon().and_then(icon_data_for_window));
    let big_source = config
        .resolve_app_icon()
        .and_then(icon_data_for_app)
        .or_else(|| config.resolve_window_icon().and_then(icon_data_for_app));

    unsafe fn create_win_icon_from_data(data: &IconData) -> Option<HICON> {
        let mut bitmap_header: BITMAPV5HEADER = std::mem::zeroed();
        bitmap_header.bV5Size = std::mem::size_of::<BITMAPV5HEADER>() as _;
        bitmap_header.bV5Width = data.width as i32;
        bitmap_header.bV5Height = -(data.height as i32);
        bitmap_header.bV5Planes = 1;
        bitmap_header.bV5BitCount = 32;
        bitmap_header.bV5Compression = BI_BITFIELDS;
        bitmap_header.bV5RedMask = 0x00FF0000;
        bitmap_header.bV5GreenMask = 0x0000FF00;
        bitmap_header.bV5BlueMask = 0x000000FF;
        bitmap_header.bV5AlphaMask = 0xFF000000;

        let mut target = std::ptr::null_mut();
        let dc = GetDC(std::ptr::null_mut());
        let color = CreateDIBSection(
            dc,
            &bitmap_header as *const _ as *const BITMAPINFO,
            DIB_RGB_COLORS,
            &mut target,
            std::ptr::null_mut(),
            0,
        );
        ReleaseDC(std::ptr::null_mut(), dc);
        if color.is_null() || target.is_null() {
            return None;
        }

        let mask = CreateBitmap(data.width as _, data.height as _, 1, 1, std::ptr::null());
        if mask.is_null() {
            DeleteObject(color as *mut _);
            return None;
        }

        for i in 0..data.width as usize * data.height as usize {
            *(target as *mut u8).add(i * 4) = data.rgba[i * 4 + 2];
            *(target as *mut u8).add(i * 4 + 1) = data.rgba[i * 4 + 1];
            *(target as *mut u8).add(i * 4 + 2) = data.rgba[i * 4];
            *(target as *mut u8).add(i * 4 + 3) = data.rgba[i * 4 + 3];
        }

        let mut icon_info: ICONINFO = std::mem::zeroed();
        icon_info.fIcon = 1;
        icon_info.hbmMask = mask;
        icon_info.hbmColor = color;
        let icon_handle = CreateIconIndirect(&mut icon_info);
        DeleteObject(color as *mut _);
        DeleteObject(mask as *mut _);

        (icon_handle != std::ptr::null_mut()).then_some(icon_handle)
    }

    fn select_closest_icon<'a>(
        preferred_px: i32,
        primary: Option<&'a IconData>,
        fallback: Option<&'a IconData>,
    ) -> Option<&'a IconData> {
        primary.or(fallback).map(|data| {
            let _ = preferred_px;
            data
        })
    }

    let small_preferred = (unsafe { GetSystemMetrics(SM_CXSMICON) }
        * unsafe { GetSystemMetrics(SM_CYSMICON) })
        .max(1);
    let big_preferred =
        (unsafe { GetSystemMetrics(SM_CXICON) } * unsafe { GetSystemMetrics(SM_CYICON) }).max(1);

    let small_icon = select_closest_icon(small_preferred, small_source, big_source)
        .and_then(|data| unsafe { create_win_icon_from_data(data) });
    let big_icon = select_closest_icon(big_preferred, big_source, small_source)
        .and_then(|data| unsafe { create_win_icon_from_data(data) });

    unsafe {
        if let Some(icon) = small_icon {
            SendMessageW(hwnd, WM_SETICON, ICON_SMALL as usize, icon as LPARAM);
        }
        if let Some(icon) = big_icon {
            SendMessageW(hwnd, WM_SETICON, ICON_BIG as usize, icon as LPARAM);
        }
    }
}

#[cfg(target_os = "macos")]
fn apply_macos_icons(window: &Arc<Window>, config: &WindowConfig) {
    use objc2::ClassType;
    use objc2::rc::Retained;
    use objc2::runtime::AnyObject;
    use objc2_app_kit::{NSApplication, NSImage, NSView};
    use objc2_foundation::{NSData, MainThreadMarker};

    fn image_from_icon(icon: &WindowIcon, prefer_large: bool) -> Option<Retained<NSImage>> {
        let png_bytes = icon_png_bytes(icon, prefer_large)?;
        let data = unsafe {
            NSData::dataWithBytes_length(
                png_bytes.as_ptr().cast_mut().cast(),
                png_bytes.len(),
            )
        };
        NSImage::initWithData(NSImage::alloc(), &data)
    }

    if let Some(icon) = config.resolve_app_icon().and_then(|icon| image_from_icon(icon, true)) {
        if let Some(mtm) = MainThreadMarker::new() {
            let app = NSApplication::sharedApplication(mtm);
            unsafe { app.setApplicationIconImage(Some(&icon)) };
        }
    }

    let Some(icon) = config
        .resolve_window_icon()
        .and_then(|icon| image_from_icon(icon, false))
    else {
        return;
    };

    let Ok(handle) = window.window_handle() else {
        return;
    };
    let RawWindowHandle::AppKit(handle) = handle.as_raw() else {
        return;
    };
    let Some(ns_view) =
        (unsafe { Retained::<AnyObject>::retain(handle.ns_view.as_ptr().cast()) })
    else {
        return;
    };
    let ns_view: Retained<NSView> = unsafe { Retained::cast(ns_view) };
    let Some(ns_window) = ns_view.window() else {
        return;
    };

    unsafe { ns_window.setMiniwindowImage(Some(icon.as_ref())) };
}
