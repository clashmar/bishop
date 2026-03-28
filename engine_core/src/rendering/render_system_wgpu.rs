// engine_core/src/rendering/render_system_wgpu.rs

use crate::prelude::*;
use bishop::prelude::*;

/// Max lights per layer.
pub const MAX_LIGHTS: usize = 10;

/// Render system that draws the game scene to an offscreen target and scales it to the window.
/// Multi-pass lighting is temporarily disabled.
pub struct RenderSystem {
    /// Current render target width
    pub rt_width: f32,
    /// Current render target height
    pub rt_height: f32,
    /// Time spent rendering last frame (ms)
    pub render_time_ms: f32,
    /// Offscreen render target at virtual resolution for scene rendering.
    scene_rt: Option<BishopRenderTarget>,
}

impl RenderSystem {
    /// Create a new render system with the given dimensions.
    fn new(width: f32, height: f32) -> Self {
        Self {
            rt_width: width,
            rt_height: height,
            render_time_ms: 0.0,
            scene_rt: None,
        }
    }

    /// Create a new render system with render targets sized for the given grid size.
    pub fn with_grid_size(grid_size: f32) -> Self {
        let width = world_virtual_width(grid_size);
        let height = world_virtual_height(grid_size);
        Self::new(width, height)
    }

    /// Create a new render system with default grid size (16.0).
    pub fn with_default_grid_size() -> Self {
        Self::with_grid_size(16.0)
    }

    /// Begins rendering to the offscreen scene render target at virtual resolution.
    /// Lazily creates or resizes the render target as needed.
    pub fn begin_scene<C: BishopContext>(&mut self, ctx: &mut C) {
        let w = self.rt_width as u32;
        let h = self.rt_height as u32;

        let needs_create = match &self.scene_rt {
            Some(rt) => rt.width() != w || rt.height() != h,
            None => true,
        };

        if needs_create {
            self.scene_rt = Some(ctx.create_drawable_render_target(w, h));
        }

        if let Some(rt) = &self.scene_rt {
            ctx.begin_render_to_target(rt);
        }
    }

    /// Ends rendering to the offscreen scene render target.
    pub fn end_scene<C: BishopContext>(&self, ctx: &mut C) {
        if self.scene_rt.is_some() {
            ctx.end_render_to_target();
        }
    }

    /// Returns the letterboxed viewport rect for the current window size.
    pub fn viewport_rect<C: BishopContext>(&self, ctx: &C) -> Rect {
        let virt_w = self.rt_width;
        let virt_h = self.rt_height;
        let win_w = ctx.screen_width();
        let win_h = ctx.screen_height();

        let scale = (win_w / virt_w).min(win_h / virt_h);
        let scaled_w = virt_w * scale;
        let scaled_h = virt_h * scale;

        Rect::new(
            ((win_w - scaled_w) / 2.0).floor(),
            ((win_h - scaled_h) / 2.0).floor(),
            scaled_w,
            scaled_h,
        )
    }

    /// Presents the scene render target scaled to the window with aspect-ratio-preserving letterboxing.
    pub fn present_game<C: BishopContext>(&self, ctx: &mut C) {
        let Some(rt) = &self.scene_rt else {
            return;
        };

        ctx.set_default_camera();

        let vp = self.viewport_rect(ctx);
        ctx.draw_render_target(rt, vp.x, vp.y, vp.w, vp.h);
    }

    /// Re-creates every render target with the supplied size.
    pub fn resize(&mut self, _width: u32, _height: u32) {
        // TODO: Re-implement
        // self.rt_width = width as f32;
        // self.rt_height = height as f32;

        // let make = || {
        //     let rt = render_target(width, height);
        //     rt.texture.set_filter(FilterMode::Nearest);
        //     rt
        // };

        // self.scene_rt = make();
        // self.ambient_rt = make();
        // self.glow_rt = make();
        // self.undarkened_rt = make();
        // self.spot_rt = make();
        // self.mask_rt = make();
        // self.scene_comp_rt = make();
        // self.final_comp_rt = make();

        // // Reset the mask cam
        // self.init_mask_cam();
    }

    /// Resizes render targets to match the camera zoom.
    pub fn resize_for_camera(&mut self, zoom: Vec2) {
        let required_width = (2.0 / zoom.x).round();
        let required_height = (2.0 / zoom.y).round();

        if required_width != self.rt_width || required_height != self.rt_height {
            self.rt_width = required_width;
            self.rt_height = required_height;
        }
    }

    /// Resizes render targets to match window size if they don't already.
    pub fn resize_to_window<C: BishopContext>(&mut self, ctx: &mut C) {
        let win_w = ctx.screen_width();
        let win_h = ctx.screen_height();

        if win_w != self.rt_width || win_h != self.rt_height {
            self.rt_width = win_w;
            self.rt_height = win_h;
        }
    }
}

impl Default for RenderSystem {
    fn default() -> Self {
        Self::with_default_grid_size()
    }
}
