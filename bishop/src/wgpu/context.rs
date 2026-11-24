//! WgpuContext main struct.

use std::sync::Arc;
use winit::event::{ElementState, KeyEvent, MouseScrollDelta, WindowEvent};
use winit::keyboard::PhysicalKey;
use winit::window::{Fullscreen, Window};

use super::conversions::{
    convert_cursor_icon, convert_keycode, convert_mouse_button, keycode_to_char,
};
use super::exec::FrameFuture;
use super::render::{
    create_texture_bind_group_layout, BishopRenderTarget, CameraUniforms, FullscreenQuadRenderer,
    LightingRenderer, PrimitiveRenderer, TextRenderer, TextureRenderer,
};
use super::state::{GraphicsState, GraphicsStateError, TimeState};
use crate::Rect;
use crate::camera::Camera2D;
use crate::input::{KeyboardMouseState, MouseWheelKind};
use crate::types::Color;
use crate::window::CursorIcon;

/// A recorded draw call, capturing which renderer and which vertex/batch range to replay.
enum DrawSegment {
    Primitive {
        start: u32,
        count: u32,
    },
    Texture {
        batch_start: usize,
        batch_count: usize,
    },
    Text {
        start: u32,
        count: u32,
    },
}

/// Wgpu backend implementation for bishop.
pub struct WgpuContext {
    pub(crate) graphics: GraphicsState,
    pub(crate) input: KeyboardMouseState,
    pub(crate) time: TimeState,
    pub(crate) window: Arc<Window>,
    pub(crate) clear_color: Option<Color>,
    pub(crate) primitive_renderer: PrimitiveRenderer,
    pub(crate) texture_renderer: TextureRenderer,
    pub(crate) text_renderer: TextRenderer,
    pub(crate) current_camera: Option<Camera2D>,
    render_target_bind_group_layout: std::sync::Arc<wgpu::BindGroupLayout>,
    fullscreen_quad_renderer: FullscreenQuadRenderer,
    lighting_renderer: LightingRenderer,
    fullscreen: bool,
    scale_factor: f32,
    current_surface_texture: Option<wgpu::SurfaceTexture>,
    current_surface_view: Option<wgpu::TextureView>,
    has_cleared_this_frame: bool,
    saved_has_cleared: bool,
    saved_surface_view: Option<wgpu::TextureView>,
    render_target_dims: Option<(f32, f32)>,
    clip_rect: Option<[u32; 4]>,
    logical_clip_rect: Option<Rect>,
    draw_segments: Vec<DrawSegment>,
    pub(crate) close_requested: bool,
    pub(crate) exit_confirmed: bool,
}

impl WgpuContext {
    /// Creates a new wgpu context from a window.
    pub async fn new(window: Arc<Window>) -> Result<Self, GraphicsStateError> {
        let graphics = GraphicsState::new(window.clone()).await?;

        let primitive_renderer = PrimitiveRenderer::new(&graphics.device, graphics.config.format);
        let texture_renderer = TextureRenderer::new(&graphics.device, graphics.config.format);
        let text_renderer =
            TextRenderer::new(&graphics.device, &graphics.queue, graphics.config.format);

        let render_target_bind_group_layout =
            std::sync::Arc::new(create_texture_bind_group_layout(&graphics.device));
        let fullscreen_quad_renderer = FullscreenQuadRenderer::new(&graphics.device);
        let lighting_renderer = LightingRenderer::new(
            &graphics.device,
            graphics.config.format,
            &fullscreen_quad_renderer,
        );
        let scale_factor = window.scale_factor() as f32;
        let fullscreen = window.fullscreen().is_some();

        Ok(Self {
            graphics,
            input: KeyboardMouseState::new(),
            time: TimeState::new(),
            window,
            clear_color: None,
            primitive_renderer,
            texture_renderer,
            text_renderer,
            current_camera: None,
            render_target_bind_group_layout,
            fullscreen_quad_renderer,
            lighting_renderer,
            fullscreen,
            scale_factor,
            current_surface_texture: None,
            current_surface_view: None,
            has_cleared_this_frame: false,
            saved_has_cleared: false,
            saved_surface_view: None,
            render_target_dims: None,
            clip_rect: None,
            logical_clip_rect: None,
            draw_segments: Vec::new(),
            close_requested: false,
            exit_confirmed: false,
        })
    }

    /// Creates a new wgpu context synchronously using pollster.
    pub fn new_sync(window: Arc<Window>) -> Result<Self, GraphicsStateError> {
        pollster::block_on(Self::new(window))
    }

    /// Prepares for a new frame by clearing per-frame state.
    pub fn begin_frame(&mut self) {
        let frame_started_at = self.time.begin_redraw();

        // Proactively resize if the window size changed (e.g. fullscreen transition)
        // before the Resized event arrives, avoiding stale-size reconfigures.
        let inner = self.window.inner_size();
        let (cw, ch) = self.graphics.size;
        if (inner.width, inner.height) != (cw, ch) {
            self.graphics.resize(inner.width, inner.height);
        }

        match self.graphics.surface.get_current_texture() {
            Ok(texture) => {
                self.time.set_acquire_wait(frame_started_at);
                let view = texture
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default());
                self.current_surface_texture = Some(texture);
                self.current_surface_view = Some(view);
            }
            Err(wgpu::SurfaceError::Outdated | wgpu::SurfaceError::Lost) => {
                self.graphics.reconfigure();
                match self.graphics.surface.get_current_texture() {
                    Ok(texture) => {
                        self.time.set_acquire_wait(frame_started_at);
                        let view = texture
                            .texture
                            .create_view(&wgpu::TextureViewDescriptor::default());
                        self.current_surface_texture = Some(texture);
                        self.current_surface_view = Some(view);
                    }
                    Err(e) => {
                        self.time.set_acquire_wait(frame_started_at);
                        eprintln!("Failed to acquire surface texture after reconfigure: {e}");
                        self.current_surface_texture = None;
                        self.current_surface_view = None;
                    }
                }
            }
            Err(e) => {
                self.time.set_acquire_wait(frame_started_at);
                eprintln!("Failed to acquire surface texture: {e}");
                self.current_surface_texture = None;
                self.current_surface_view = None;
            }
        }

        self.time.begin_frame();

        self.input.begin_frame();

        // Clear state
        self.clear_color = None;
        self.primitive_renderer.clear();
        self.texture_renderer.clear();
        self.text_renderer.clear();
        self.draw_segments.clear();
        self.current_camera = None;
        self.has_cleared_this_frame = false;
        self.clip_rect = None;
        self.logical_clip_rect = None;
    }

    /// Processes a winit WindowEvent and updates internal state.
    pub fn handle_window_event(&mut self, event: &WindowEvent) {
        match event {
            WindowEvent::Resized(size) => {
                self.graphics.resize(size.width, size.height);
            }
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                self.scale_factor = *scale_factor as f32;
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(keycode),
                        state,
                        text,
                        ..
                    },
                ..
            } => {
                let key = convert_keycode(*keycode);
                match state {
                    ElementState::Pressed => {
                        self.input.on_key_down(key);
                        let mut got_text = false;
                        if let Some(txt) = text {
                            for c in txt.chars() {
                                if !c.is_control() {
                                    self.input.on_char(c);
                                    got_text = true;
                                }
                            }
                        }
                        if !got_text {
                            let shift = self.input.is_key_down(crate::input::KeyCode::LeftShift)
                                || self.input.is_key_down(crate::input::KeyCode::RightShift);
                            if let Some(c) = keycode_to_char(*keycode, shift) {
                                self.input.on_char(c);
                            }
                        }
                    }
                    ElementState::Released => self.input.on_key_up(key),
                }
            }
            WindowEvent::MouseInput { button, state, .. } => {
                if let Some(btn) = convert_mouse_button(*button) {
                    match state {
                        ElementState::Pressed => {
                            let time = self.time.elapsed();
                            let pos = self.input.mouse_position();
                            self.input.on_mouse_down(btn, time, pos);
                        }
                        ElementState::Released => self.input.on_mouse_up(btn),
                    }
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                let x = position.x as f32 / self.scale_factor;
                let y = position.y as f32 / self.scale_factor;
                self.input.on_mouse_move(x, y);
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let (dx, dy, kind) = match delta {
                    MouseScrollDelta::LineDelta(x, y) => (*x, *y, MouseWheelKind::Line),
                    MouseScrollDelta::PixelDelta(pos) => (
                        pos.x as f32 / self.scale_factor,
                        pos.y as f32 / self.scale_factor,
                        MouseWheelKind::Pixel,
                    ),
                };
                self.input.on_mouse_wheel(dx, dy, kind);
            }
            _ => {}
        }
    }

    /// Returns the source kind for the most recent mouse wheel event this frame.
    pub fn mouse_wheel_kind(&self) -> Option<MouseWheelKind> {
        self.input.mouse_wheel_kind()
    }

    /// Returns a reference to the underlying window.
    pub fn window(&self) -> &Window {
        &self.window
    }

    pub fn is_close_requested(&self) -> bool {
        self.close_requested
    }

    pub fn set_close_requested(&mut self, requested: bool) {
        self.close_requested = requested;
    }

    pub fn is_exit_confirmed(&self) -> bool {
        self.exit_confirmed
    }

    pub fn set_exit_confirmed(&mut self, confirmed: bool) {
        self.exit_confirmed = confirmed;
    }

    /// Returns the current screen width in logical pixels.
    /// When rendering to a target, returns the target width instead.
    pub fn screen_width(&self) -> f32 {
        if let Some((w, _)) = self.render_target_dims {
            return w;
        }
        self.graphics.size.0 as f32 / self.scale_factor
    }

    /// Returns the font rasterization scale for the current display and camera.
    pub(crate) fn text_raster_scale(&self) -> f32 {
        let display_scale = if self.render_target_dims.is_some() {
            1.0
        } else {
            self.scale_factor
        };
        let Some(camera) = &self.current_camera else {
            return display_scale.max(0.01);
        };
        let (viewport_width, viewport_height) = camera
            .viewport
            .map(|(_, _, width, height)| (width as f32, height as f32))
            .unwrap_or_else(|| (self.screen_width(), self.screen_height()));
        crate::text::raster_scale_for_camera(
            display_scale,
            viewport_width,
            viewport_height,
            camera.zoom.x,
            camera.zoom.y,
        )
    }

    /// Returns the current screen height in logical pixels.
    /// When rendering to a target, returns the target height instead.
    pub fn screen_height(&self) -> f32 {
        if let Some((_, h)) = self.render_target_dims {
            return h;
        }
        self.graphics.size.1 as f32 / self.scale_factor
    }

    /// Returns the current screen width in physical pixels.
    pub fn physical_width(&self) -> u32 {
        self.graphics.size.0
    }

    /// Returns the current screen height in physical pixels.
    pub fn physical_height(&self) -> u32 {
        self.graphics.size.1
    }

    /// Sets the mouse cursor icon.
    pub fn set_cursor_icon(&mut self, icon: CursorIcon) {
        self.window.set_cursor(convert_cursor_icon(icon));
    }

    /// Toggles fullscreen mode and returns the new state.
    pub fn toggle_fullscreen(&mut self) -> bool {
        self.fullscreen = !self.fullscreen;
        let fullscreen_mode = if self.fullscreen {
            Some(Fullscreen::Borderless(None))
        } else {
            None
        };
        self.window.set_fullscreen(fullscreen_mode);
        let size = self.window.inner_size();
        self.graphics.resize(size.width, size.height);
        self.fullscreen
    }

    /// Returns whether the window is currently in fullscreen mode.
    pub fn is_fullscreen(&self) -> bool {
        self.fullscreen
    }

    /// Returns the display scale factor (DPI scaling).
    pub fn scale_factor(&self) -> f32 {
        self.scale_factor
    }

    /// Restricts subsequent rendering to the given logical-pixel rectangle.
    /// Flushes any pending draws first so pre-clip content renders without the scissor.
    /// Must be paired with [`pop_clip_rect`](Self::pop_clip_rect) to restore full-screen rendering.
    pub fn push_clip_rect(&mut self, rect: crate::types::Rect) {
        self.flush_if_needed();
        let sf = self.scale_factor;
        self.clip_rect = Some([
            (rect.x * sf) as u32,
            (rect.y * sf) as u32,
            (rect.w * sf).max(0.0) as u32,
            (rect.h * sf).max(0.0) as u32,
        ]);
        self.logical_clip_rect = Some(rect);
    }

    /// Flushes pending draws with the active scissor applied, then removes the clip rect.
    pub fn pop_clip_rect(&mut self) {
        self.flush_if_needed();
        self.clip_rect = None;
        self.logical_clip_rect = None;
    }

    /// Returns the active logical-pixel clip rect, or `None` when rendering is unclipped.
    pub(crate) fn logical_clip_rect(&self) -> Option<Rect> {
        self.logical_clip_rect
    }

    /// Returns the texture bind group layout for creating textures.
    pub fn texture_bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        self.texture_renderer.texture_bind_group_layout()
    }

    /// Returns a reference to the wgpu device.
    pub fn device(&self) -> &wgpu::Device {
        &self.graphics.device
    }

    /// Returns the Arc-wrapped wgpu device for shared ownership.
    pub fn device_arc(&self) -> Arc<wgpu::Device> {
        self.graphics.device.clone()
    }

    /// Returns a reference to the wgpu queue.
    pub fn queue(&self) -> &wgpu::Queue {
        &self.graphics.queue
    }

    /// Returns the Arc-wrapped wgpu queue for shared ownership.
    pub fn queue_arc(&self) -> Arc<wgpu::Queue> {
        self.graphics.queue.clone()
    }

    /// Returns the Arc-wrapped texture bind group layout for shared ownership.
    pub fn texture_bind_group_layout_arc(&self) -> Arc<wgpu::BindGroupLayout> {
        self.texture_renderer.texture_bind_group_layout_arc()
    }

    /// Returns the surface format for pipeline creation.
    pub fn surface_format(&self) -> wgpu::TextureFormat {
        self.graphics.config.format
    }

    /// Creates a render target with the specified dimensions.
    pub fn create_render_target(&self, width: u32, height: u32) -> BishopRenderTarget {
        BishopRenderTarget::new(
            &self.graphics.device,
            self.render_target_bind_group_layout.clone(),
            width,
            height,
            self.graphics.config.format,
        )
    }

    /// Returns the render target bind group layout for creating render targets externally.
    pub fn render_target_bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.render_target_bind_group_layout
    }

    /// Returns the Arc-wrapped render target bind group layout for shared ownership.
    pub fn render_target_bind_group_layout_arc(&self) -> Arc<wgpu::BindGroupLayout> {
        self.render_target_bind_group_layout.clone()
    }

    /// Returns a reference to the fullscreen quad renderer.
    pub fn fullscreen_quad_renderer(&self) -> &FullscreenQuadRenderer {
        &self.fullscreen_quad_renderer
    }

    /// Returns a reference to the current surface texture view if available.
    /// Used for direct rendering passes that bypass the batched rendering system.
    pub fn current_surface_view(&self) -> Option<&wgpu::TextureView> {
        self.current_surface_view.as_ref()
    }

    /// Returns whether the frame has been cleared yet.
    pub fn has_cleared_this_frame(&self) -> bool {
        self.has_cleared_this_frame
    }

    /// Marks the frame as cleared. Call this after performing a clear operation.
    pub fn mark_cleared(&mut self) {
        self.has_cleared_this_frame = true;
    }

    /// Returns the current clear color, if set.
    pub fn clear_color(&self) -> Option<Color> {
        self.clear_color
    }

    /// Returns a future that completes on the next frame.
    /// Use this to yield from async code and allow the frame to render.
    pub fn next_frame(&self) -> FrameFuture {
        FrameFuture::default()
    }

    /// Clears per-frame input state (pressed/released events).
    /// Called by app_runner when a frame completes.
    pub fn end_frame_input(&mut self) {
        self.input.end_frame();
    }

    /// Returns true if any renderer has pending draw calls.
    fn has_pending_draws(&self) -> bool {
        !self.primitive_renderer.is_empty()
            || !self.texture_renderer.is_empty()
            || !self.text_renderer.is_empty()
    }

    /// Records a primitive draw segment, merging with the previous if it was also primitive.
    /// Also seals the texture batch so the next texture draw starts fresh.
    pub(super) fn record_primitive_segment(&mut self, prev: u32) {
        let current = self.primitive_renderer.vertex_count() as u32;
        let count = current - prev;
        if count == 0 {
            return;
        }
        self.texture_renderer.seal_batch();
        match self.draw_segments.last_mut() {
            Some(DrawSegment::Primitive { count: c, .. }) => *c += count,
            _ => self
                .draw_segments
                .push(DrawSegment::Primitive { start: prev, count }),
        }
    }

    /// Records a texture draw segment, merging with the previous if it was also texture.
    pub(super) fn record_texture_segment(&mut self, prev: usize) {
        let current = self.texture_renderer.batch_count();
        let added = current - prev;
        match self.draw_segments.last_mut() {
            Some(DrawSegment::Texture { batch_count, .. }) => *batch_count += added,
            _ if added > 0 => self.draw_segments.push(DrawSegment::Texture {
                batch_start: prev,
                batch_count: added,
            }),
            _ => {}
        }
    }

    /// Records a text draw segment, merging with the previous if it was also text.
    /// Also seals the texture batch so the next texture draw starts fresh.
    pub(super) fn record_text_segment(&mut self, prev: u32) {
        let current = self.text_renderer.vertex_count() as u32;
        let count = current - prev;
        if count == 0 {
            return;
        }
        self.texture_renderer.seal_batch();
        match self.draw_segments.last_mut() {
            Some(DrawSegment::Text { count: c, .. }) => *c += count,
            _ => self
                .draw_segments
                .push(DrawSegment::Text { start: prev, count }),
        }
    }

    /// Flushes batched draw calls in recorded order with the current camera state.
    fn flush_batched(&mut self, view: &wgpu::TextureView, load_op: wgpu::LoadOp<wgpu::Color>) {
        let width = self.screen_width();
        let height = self.screen_height();

        let uniforms = if let Some(camera) = &self.current_camera {
            CameraUniforms::from_camera2d(camera, width, height)
        } else {
            CameraUniforms::screen_space(width, height)
        };

        self.primitive_renderer
            .update_uniforms(&self.graphics.queue, &uniforms);
        self.texture_renderer
            .update_uniforms(&self.graphics.queue, &uniforms);
        self.text_renderer
            .update_uniforms(&self.graphics.queue, &uniforms);

        // Atlas upload requires &mut and must happen before the render pass.
        self.text_renderer.upload_atlas(&self.graphics.queue);

        // Upload all vertex buffers before opening the render pass.
        self.primitive_renderer
            .upload_vertices(&self.graphics.queue);
        self.texture_renderer.upload_vertices(&self.graphics.queue);
        self.text_renderer.upload_vertices(&self.graphics.queue);

        let mut encoder =
            self.graphics
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("flush_encoder"),
                });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("flush_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: load_op,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            if let Some([x, y, w, h]) = self.clip_rect {
                let (pw, ph) = self.graphics.size;
                let x = x.min(pw);
                let y = y.min(ph);
                render_pass.set_scissor_rect(
                    x,
                    y,
                    w.min(pw.saturating_sub(x)),
                    h.min(ph.saturating_sub(y)),
                );
            }

            if let Some(camera) = &self.current_camera {
                if let Some((x, y, w, h)) = camera.viewport {
                    let scale = camera_viewport_scale(
                        self.scale_factor,
                        self.render_target_dims.is_some(),
                    );
                    render_pass.set_viewport(
                        x as f32 * scale,
                        y as f32 * scale,
                        w as f32 * scale,
                        h as f32 * scale,
                        0.0,
                        1.0,
                    );
                }
            }

            for segment in &self.draw_segments {
                match segment {
                    DrawSegment::Primitive { start, count } => {
                        self.primitive_renderer.setup_pipeline(&mut render_pass);
                        self.primitive_renderer
                            .draw_range(&mut render_pass, *start, *count);
                    }
                    DrawSegment::Texture {
                        batch_start,
                        batch_count,
                    } => {
                        self.texture_renderer.setup_pipeline(&mut render_pass);
                        self.texture_renderer.draw_batches_range(
                            &mut render_pass,
                            *batch_start,
                            *batch_count,
                        );
                    }
                    DrawSegment::Text { start, count } => {
                        if self.text_renderer.setup_pipeline(&mut render_pass) {
                            self.text_renderer
                                .draw_range(&mut render_pass, *start, *count);
                        }
                    }
                }
            }

            if self.clip_rect.is_some() {
                let (pw, ph) = self.graphics.size;
                render_pass.set_scissor_rect(0, 0, pw, ph);
            }
        }

        self.graphics
            .queue
            .submit(std::iter::once(encoder.finish()));

        self.primitive_renderer.clear();
        self.texture_renderer.clear();
        self.text_renderer.clear();
        self.draw_segments.clear();
    }

    /// Flushes pending draws if there are any batched vertices.
    /// Called before camera changes to ensure draws use the correct transform.
    /// Also used by external renderers (like GridRenderer) to ensure batched draws
    /// are flushed before direct rendering.
    pub fn flush_if_needed(&mut self) {
        if !self.has_pending_draws() {
            return;
        }

        let Some(view) = self.current_surface_view.take() else {
            return;
        };

        let load_op = if self.has_cleared_this_frame {
            wgpu::LoadOp::Load
        } else {
            let clear_color = self.clear_color.unwrap_or(Color::BLACK);
            self.has_cleared_this_frame = true;
            wgpu::LoadOp::Clear(wgpu::Color {
                r: clear_color.r as f64,
                g: clear_color.g as f64,
                b: clear_color.b as f64,
                a: clear_color.a as f64,
            })
        };

        self.flush_batched(&view, load_op);

        self.current_surface_view = Some(view);
    }

    /// Redirects rendering to an offscreen render target.
    /// All subsequent draw calls will render into the target until `end_render_to_target` is called.
    pub fn begin_render_to_target(&mut self, rt: &BishopRenderTarget) {
        self.flush_if_needed();
        self.saved_has_cleared = self.has_cleared_this_frame;
        self.saved_surface_view = self.current_surface_view.take();
        self.current_surface_view = Some(rt.render_view().clone());
        self.render_target_dims = Some((rt.width() as f32, rt.height() as f32));
        self.has_cleared_this_frame = false;
    }

    /// Stops rendering to the offscreen target and restores the screen surface.
    pub fn end_render_to_target(&mut self) {
        self.flush_if_needed();
        self.current_surface_view = self.saved_surface_view.take();
        self.render_target_dims = None;
        self.has_cleared_this_frame = self.saved_has_cleared;
    }

    /// Draws a render target's contents as a textured quad at the given position and size.
    pub fn draw_render_target(
        &mut self,
        rt: &BishopRenderTarget,
        x: f32,
        y: f32,
        dest_w: f32,
        dest_h: f32,
    ) {
        let prev = self.texture_renderer.batch_count();
        self.texture_renderer
            .draw_render_target_quad(rt.bind_group(), x, y, dest_w, dest_h);
        self.record_texture_segment(prev);
    }

    /// Creates a render target whose bind group is compatible with the texture renderer pipeline.
    /// Use this for render targets that will be drawn via `draw_render_target`.
    pub fn create_drawable_render_target(&self, width: u32, height: u32) -> BishopRenderTarget {
        BishopRenderTarget::new(
            &self.graphics.device,
            self.texture_renderer.texture_bind_group_layout_arc(),
            width,
            height,
            self.graphics.config.format,
        )
    }

    pub(crate) fn apply_ambient_pass(
        &mut self,
        source: &BishopRenderTarget,
        destination: &BishopRenderTarget,
        params: crate::material::AmbientPassParams,
    ) -> Result<(), String> {
        validate_lighting_targets(source, destination)?;
        self.flush_if_needed();
        let mut encoder = self.graphics.device.create_command_encoder(
            &wgpu::CommandEncoderDescriptor { label: Some("ambient_lighting_pass") },
        );
        self.lighting_renderer.ambient(
            &self.graphics.device,
            &self.graphics.queue,
            &mut encoder,
            &self.fullscreen_quad_renderer,
            source,
            destination,
            params,
        );
        self.graphics.queue.submit(std::iter::once(encoder.finish()));
        Ok(())
    }

    pub(crate) fn apply_point_light_pass(
        &mut self,
        source: &BishopRenderTarget,
        destination: &BishopRenderTarget,
        params: crate::material::PointLightPassParams,
    ) -> Result<(), String> {
        validate_lighting_targets(source, destination)?;
        self.flush_if_needed();
        let mut encoder = self.graphics.device.create_command_encoder(
            &wgpu::CommandEncoderDescriptor { label: Some("point_light_pass") },
        );
        self.lighting_renderer.point(
            &self.graphics.device,
            &self.graphics.queue,
            &mut encoder,
            &self.fullscreen_quad_renderer,
            source,
            destination,
            params,
        );
        self.graphics.queue.submit(std::iter::once(encoder.finish()));
        Ok(())
    }

    pub(crate) fn apply_composite_lighting_pass(
        &mut self,
        albedo: &BishopRenderTarget,
        illumination: &BishopRenderTarget,
        destination: &BishopRenderTarget,
    ) -> Result<(), String> {
        validate_composite_lighting_targets(albedo, illumination, destination)?;
        self.flush_if_needed();
        let mut encoder = self.graphics.device.create_command_encoder(
            &wgpu::CommandEncoderDescriptor { label: Some("room_lighting_composite_pass") },
        );
        self.lighting_renderer.composite(
            &self.graphics.device,
            &self.graphics.queue,
            &mut encoder,
            &self.fullscreen_quad_renderer,
            albedo,
            illumination,
            destination,
        );
        self.graphics.queue.submit(std::iter::once(encoder.finish()));
        Ok(())
    }

    /// Returns the actual window width in logical pixels, ignoring any render target override.
    pub fn window_width(&self) -> f32 {
        self.graphics.size.0 as f32 / self.scale_factor
    }

    /// Returns the actual window height in logical pixels, ignoring any render target override.
    pub fn window_height(&self) -> f32 {
        self.graphics.size.1 as f32 / self.scale_factor
    }

    /// Renders the current frame and presents it.
    pub fn render_frame(&mut self) -> Result<(), wgpu::SurfaceError> {
        self.flush_if_needed();

        if frame_needs_surface_clear(self.has_pending_draws(), self.has_cleared_this_frame) {
            let Some(view) = self.current_surface_view.take() else {
                return Ok(());
            };

            self.has_cleared_this_frame = true;
            self.flush_batched(
                &view,
                wgpu::LoadOp::Clear(clear_wgpu_color(self.clear_color.unwrap_or(Color::BLACK))),
            );
            self.current_surface_view = Some(view);
        }

        if let Some(texture) = self.current_surface_texture.take() {
            let present_started_at = std::time::Instant::now();
            texture.present();
            self.time.set_present_wait(present_started_at);
        } else {
            self.time.set_present_wait(std::time::Instant::now());
        }
        self.current_surface_view = None;

        Ok(())
    }
}

fn validate_lighting_targets(
    source: &BishopRenderTarget,
    destination: &BishopRenderTarget,
) -> Result<(), String> {
    if std::ptr::eq(source.texture(), destination.texture()) {
        return Err("lighting pass source and destination targets must differ".to_owned());
    }
    if source.width() != destination.width() || source.height() != destination.height() {
        return Err("lighting pass source and destination dimensions must match".to_owned());
    }
    Ok(())
}

fn validate_composite_lighting_targets(
    albedo: &BishopRenderTarget,
    illumination: &BishopRenderTarget,
    destination: &BishopRenderTarget,
) -> Result<(), String> {
    if std::ptr::eq(albedo.texture(), destination.texture())
        || std::ptr::eq(illumination.texture(), destination.texture())
    {
        return Err("lighting composite destination must differ from both sources".to_owned());
    }
    if albedo.width() != illumination.width()
        || albedo.height() != illumination.height()
        || albedo.width() != destination.width()
        || albedo.height() != destination.height()
    {
        return Err("lighting composite targets must have matching dimensions".to_owned());
    }
    Ok(())
}

fn camera_viewport_scale(scale_factor: f32, rendering_to_target: bool) -> f32 {
    if rendering_to_target {
        1.0
    } else {
        scale_factor.max(1.0)
    }
}

fn frame_needs_surface_clear(has_pending_draws: bool, has_cleared_this_frame: bool) -> bool {
    !has_pending_draws && !has_cleared_this_frame
}

fn clear_wgpu_color(color: Color) -> wgpu::Color {
    wgpu::Color {
        r: color.r as f64,
        g: color.g as f64,
        b: color.b as f64,
        a: color.a as f64,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn camera_viewport_uses_target_pixels_without_display_scale() {
        assert_eq!(camera_viewport_scale(2.0, true), 1.0);
    }

    #[test]
    fn camera_viewport_uses_display_scale_for_surface_rendering() {
        assert_eq!(camera_viewport_scale(2.0, false), 2.0);
        assert_eq!(camera_viewport_scale(0.75, false), 1.0);
    }

    #[test]
    fn untouched_frame_still_requires_a_surface_clear() {
        assert!(frame_needs_surface_clear(false, false));
    }

    #[test]
    fn already_cleared_frame_does_not_require_another_for_present() {
        assert!(!frame_needs_surface_clear(false, true));
    }
}
