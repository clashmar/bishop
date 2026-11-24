use std::collections::{HashMap, HashSet};

use bishop::prelude::*;
use crate::omni_error;
use crate::ecs::*;
use crate::rendering::room_layer_draw::{GlowSourceDiagnostic, GlowSourceIssueKind};
use crate::rendering::room_layer_rendering::{
    ContributionLightingPlan, LayerLightingBuffer, LightingFailure, RoomLightingContribution,
    alternate_lighting_buffer, fallback_for_lighting_failure,
};

struct ContributionTargets {
    raw: BishopRenderTarget,
    illumination_a: BishopRenderTarget,
    illumination_b: BishopRenderTarget,
    composite: BishopRenderTarget,
}

/// Render system that owns the room contribution targets and final scene compositor.
pub struct RenderSystem {
    /// Current render target width
    pub rt_width: f32,
    /// Current render target height
    pub rt_height: f32,
    /// Time spent rendering last frame (ms)
    pub render_time_ms: f32,
    /// Final scene render target at virtual resolution.
    scene_rt: Option<BishopRenderTarget>,
    /// Raw albedo, ping-pong illumination, and final lit targets per contribution.
    contribution_targets: HashMap<RoomLightingContribution, ContributionTargets>,
    /// Final output chosen for presentation.
    output_rt: Option<BishopRenderTarget>,
    /// Source failures already logged, retained to avoid one warning per frame.
    reported_glow_source_issues: HashSet<GlowSourceDiagnostic>,
}

impl RenderSystem {
    fn new(width: f32, height: f32) -> Self {
        Self {
            rt_width: width,
            rt_height: height,
            render_time_ms: 0.0,
            scene_rt: None,
            contribution_targets: HashMap::new(),
            output_rt: None,
            reported_glow_source_issues: HashSet::new(),
        }
    }

    /// Creates a render system sized for the supplied world grid size.
    pub fn with_grid_size(grid_size: f32) -> Self {
        Self::new(world_virtual_width(grid_size), world_virtual_height(grid_size))
    }

    /// Creates a render system using the default 16-unit grid size.
    pub fn with_default_grid_size() -> Self {
        Self::with_grid_size(16.0)
    }

    /// Allocates or resizes all targets before drawing any room contribution.
    pub(crate) fn prepare_room<C: BishopContext>(
        &mut self,
        ctx: &C,
        plan: &[ContributionLightingPlan],
    ) -> bool {
        self.output_rt = None;
        let width = self.rt_width.max(1.0) as u32;
        let height = self.rt_height.max(1.0) as u32;

        let scene_needs_create = self.scene_rt.as_ref().is_none_or(|target| {
            target.width() != width || target.height() != height
        });
        if scene_needs_create {
            let target = ctx.create_drawable_render_target(width, height);
            if let Some(error) = ctx.render_target_error(&target) {
                Self::report_fallback(
                    LightingFailure::FinalTargetAllocation,
                    &error,
                );
                return false;
            }
            self.scene_rt = Some(target);
        }

        self.contribution_targets.retain(|contribution, _| {
            plan.iter().any(|entry| entry.contribution == *contribution)
        });

        for entry in plan {
            let needs_create = self.contribution_targets.get(&entry.contribution).is_none_or(
                |targets| {
                    targets.raw.width() != width
                        || targets.raw.height() != height
                        || targets.illumination_a.width() != width
                        || targets.illumination_a.height() != height
                        || targets.illumination_b.width() != width
                        || targets.illumination_b.height() != height
                        || targets.composite.width() != width
                        || targets.composite.height() != height
                },
            );
            if needs_create {
                let raw = ctx.create_drawable_render_target(width, height);
                if let Some(error) = ctx.render_target_error(&raw) {
                    Self::report_fallback(LightingFailure::LayerTargetAllocation, &error);
                    return false;
                }
                let illumination_a = ctx.create_drawable_render_target(width, height);
                if let Some(error) = ctx.render_target_error(&illumination_a) {
                    Self::report_fallback(LightingFailure::LayerTargetAllocation, &error);
                    return false;
                }
                let illumination_b = ctx.create_drawable_render_target(width, height);
                if let Some(error) = ctx.render_target_error(&illumination_b) {
                    Self::report_fallback(LightingFailure::LayerTargetAllocation, &error);
                    return false;
                }
                let composite = ctx.create_drawable_render_target(width, height);
                if let Some(error) = ctx.render_target_error(&composite) {
                    Self::report_fallback(LightingFailure::LayerTargetAllocation, &error);
                    return false;
                }
                self.contribution_targets.insert(
                    entry.contribution,
                    ContributionTargets { raw, illumination_a, illumination_b, composite },
                );
            }
        }

        true
    }

    pub(crate) fn begin_contribution<C: BishopContext>(
        &self,
        ctx: &mut C,
        contribution: RoomLightingContribution,
    ) {
        if let Some(targets) = self.contribution_targets.get(&contribution) {
            ctx.begin_render_to_target(&targets.raw);
            ctx.clear_background(Color::TRANSPARENT);
        }
    }

    pub(crate) fn end_contribution<C: BishopContext>(&self, ctx: &mut C) {
        ctx.end_render_to_target();
    }

    /// Lights one isolated contribution while retaining raw albedo for the final composite.
    pub(crate) fn light_contribution<C: BishopContext>(
        &self,
        ctx: &mut C,
        contribution: RoomLightingContribution,
        ambient: AmbientPassParams,
        point_lights: &[PointLightPassParams],
    ) -> Result<LayerLightingBuffer, String> {
        let Some(targets) = self.contribution_targets.get(&contribution) else {
            return Err(format!("missing lighting targets for {contribution:?}"));
        };
        ctx.apply_ambient_pass(&targets.raw, &targets.illumination_a, ambient)?;
        let mut source = LayerLightingBuffer::A;

        for light in point_lights {
            if light.radius <= 0.0 || light.intensity <= 0.0 {
                continue;
            }
            let destination = alternate_lighting_buffer(source);
            let (source_target, destination_target) = match (source, destination) {
                (LayerLightingBuffer::A, LayerLightingBuffer::B) => {
                    (&targets.illumination_a, &targets.illumination_b)
                }
                (LayerLightingBuffer::B, LayerLightingBuffer::A) => {
                    (&targets.illumination_b, &targets.illumination_a)
                }
                _ => unreachable!("lighting ping-pong buffers must differ"),
            };
            ctx.apply_point_light_pass(source_target, destination_target, *light)?;
            source = destination;
        }
        let illumination = match source {
            LayerLightingBuffer::A => &targets.illumination_a,
            LayerLightingBuffer::B => &targets.illumination_b,
        };
        ctx.composite_lighting_pass(&targets.raw, illumination, &targets.composite)?;

        Ok(source)
    }

    /// Composites completed contribution targets in the planned room order.
    pub(crate) fn composite_room<C: BishopContext>(
        &mut self,
        ctx: &mut C,
        plan: &[ContributionLightingPlan],
        lit_buffers: &HashMap<RoomLightingContribution, LayerLightingBuffer>,
        use_lit_results: bool,
    ) -> bool {
        let Some(scene) = self.scene_rt.as_ref() else {
            Self::report_fallback(
                LightingFailure::FinalTargetAllocation,
                "final scene target is unavailable",
            );
            return false;
        };
        ctx.begin_render_to_target(scene);
        ctx.set_default_camera();
        ctx.clear_background(Color::BLACK);

        for entry in plan {
            let Some(targets) = self.contribution_targets.get(&entry.contribution) else {
                ctx.end_render_to_target();
                Self::report_fallback(
                    LightingFailure::LayerTargetAllocation,
                    "a planned contribution target is unavailable",
                );
                return false;
            };
            let target = if use_lit_results
                && entry.contribution != RoomLightingContribution::DoorGhosts
                && lit_buffers.contains_key(&entry.contribution)
            {
                &targets.composite
            } else {
                &targets.raw
            };
            ctx.draw_render_target(target, 0.0, 0.0, self.rt_width, self.rt_height);
        }

        ctx.end_render_to_target();
        self.output_rt = self.scene_rt.clone();
        true
    }

    pub(crate) fn draw_unlit_room_fallback(&mut self) {
        self.output_rt = None;
    }

    pub(crate) fn report_pass_failure(detail: &str) {
        Self::report_fallback(LightingFailure::LightingPass, detail);
    }

    pub(super) fn report_glow_source_diagnostics(
        &mut self,
        diagnostics: &[GlowSourceDiagnostic],
    ) {
        for diagnostic in diagnostics {
            if !self.reported_glow_source_issues.insert(*diagnostic) {
                continue;
            }

            let detail = match diagnostic.kind {
                GlowSourceIssueKind::DisplayedVisualUnavailable => {
                    "no drawable Sprite or CurrentFrame visual is available"
                }
                GlowSourceIssueKind::MissingCurrentFrame => {
                    "the resolved visual has no CurrentFrame for Aseprite slice selection"
                }
                GlowSourceIssueKind::MissingSliceBounds => {
                    "no valid imported glow slice bounds exist for the active clip and frame"
                }
            };
            crate::omni_warn!(
                "glow_source_unavailable entity={:?}: {}; emission skipped",
                diagnostic.entity,
                detail,
            );
        }
    }

    fn report_fallback(failure: LightingFailure, detail: &str) {
        let outcome = fallback_for_lighting_failure(failure);
        omni_error!(
            "{}: {}. Falling back to {:?} for this frame",
            outcome.diagnostic_code,
            detail,
            outcome.fallback
        );
    }

    /// Returns a letterboxed viewport rectangle for the current window.
    pub fn viewport_rect<C: BishopContext>(&self, ctx: &C) -> Rect {
        let win_w = ctx.screen_width();
        let win_h = ctx.screen_height();
        let scale = (win_w / self.rt_width).min(win_h / self.rt_height);
        let scaled_w = self.rt_width * scale;
        let scaled_h = self.rt_height * scale;
        Rect::new(
            ((win_w - scaled_w) / 2.0).floor(),
            ((win_h - scaled_h) / 2.0).floor(),
            scaled_w,
            scaled_h,
        )
    }

    /// Presents the completed room output with aspect-ratio-preserving letterboxing.
    pub fn present_game<C: BishopContext>(&self, ctx: &mut C) {
        let Some(rt) = self.output_rt.as_ref() else {
            return;
        };
        ctx.set_default_camera();
        let viewport = self.viewport_rect(ctx);
        ctx.draw_render_target(rt, viewport.x, viewport.y, viewport.w, viewport.h);
    }

    /// Recreates render targets at the supplied dimensions on the next room render.
    pub fn resize(&mut self, width: u32, height: u32) {
        self.rt_width = width.max(1) as f32;
        self.rt_height = height.max(1) as f32;
        self.scene_rt = None;
        self.contribution_targets.clear();
        self.output_rt = None;
    }

    /// Resizes render targets to match the camera's virtual resolution.
    pub fn resize_for_camera(&mut self, zoom: Vec2) {
        let required_width = (2.0 / zoom.x).round();
        let required_height = (2.0 / zoom.y).round();
        if required_width != self.rt_width || required_height != self.rt_height {
            self.resize(required_width.max(1.0) as u32, required_height.max(1.0) as u32);
        }
    }

    /// Resizes render targets to match the current window dimensions.
    pub fn resize_to_window<C: BishopContext>(&mut self, ctx: &mut C) {
        let width = ctx.screen_width().max(1.0) as u32;
        let height = ctx.screen_height().max(1.0) as u32;
        if width as f32 != self.rt_width || height as f32 != self.rt_height {
            self.resize(width, height);
        }
    }
}

impl Default for RenderSystem {
    fn default() -> Self {
        Self::with_default_grid_size()
    }
}
