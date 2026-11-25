//! Render operations trait for offscreen render targets.

use super::BishopRenderTarget;

/// Room-wide ambient values consumed by the fixed ambient pass.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AmbientPassParams {
    /// Linear RGB multiplier before darkness and exposure are applied.
    pub color: [f32; 3],
    /// Ambient exposure multiplier.
    pub exposure: f32,
    /// Room darkness in the inclusive range 0..=1.
    pub darkness: f32,
}

/// One point-light input for the fixed point-light pass.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PointLightPassParams {
    /// Light center in destination render-target pixel coordinates.
    pub center: [f32; 2],
    /// Linear RGB light color.
    pub color: [f32; 3],
    /// Light intensity multiplier.
    pub intensity: f32,
    /// Light radius in destination render-target pixels.
    pub radius: f32,
    /// Radial falloff exponent.
    pub attenuation: f32,
    /// Quantize light falloff sampling to a pixel grid when enabled.
    pub pixelated: bool,
    /// Light-sampling grid spacing in destination render-target pixels.
    pub pixel_size: f32,
    /// World-origin position in destination render-target pixel coordinates.
    pub pixel_grid_origin: [f32; 2],
}

/// One sprite-emission input for the fixed emission pass.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EmissionPassParams {
    /// Linear RGB emission color.
    pub color: [f32; 3],
    /// Emission intensity multiplier.
    pub intensity: f32,
    /// Halo sampling spread in destination render-target pixels.
    pub halo_spread: f32,
    /// Quantize emission-mask and halo sampling to a pixel grid when enabled.
    pub pixelated: bool,
    /// Pixel-grid spacing in destination render-target pixels.
    pub pixel_size: f32,
    /// World-origin position in destination render-target pixel coordinates.
    pub pixel_grid_origin: [f32; 2],
}

/// Returns whether a pass uses separate source-mask, illumination, and output handles.
pub fn emission_pass_targets_are_distinct<T>(
    source_mask: &T,
    illumination: &T,
    destination: &T,
) -> bool {
    !std::ptr::eq(source_mask, illumination)
        && !std::ptr::eq(source_mask, destination)
        && !std::ptr::eq(illumination, destination)
}

/// Trait for render target operations (offscreen rendering and drawing).
pub trait RenderOps {
    /// Redirects rendering to an offscreen render target.
    fn begin_render_to_target(&mut self, rt: &BishopRenderTarget);

    /// Stops rendering to the offscreen target and restores the screen surface.
    fn end_render_to_target(&mut self);

    /// Draws a render target's contents as a textured quad at the given position and size.
    fn draw_render_target(&mut self, rt: &BishopRenderTarget, x: f32, y: f32, w: f32, h: f32);

    /// Initializes encoded room ambient irradiance in a distinct destination target.
    fn apply_ambient_pass(
        &mut self,
        source: &BishopRenderTarget,
        destination: &BishopRenderTarget,
        params: AmbientPassParams,
    ) -> Result<(), String>;

    /// Adds one sprite-shaped emission source and its halo to encoded illumination.
    fn apply_emission_pass(
        &mut self,
        source_mask: &BishopRenderTarget,
        illumination: &BishopRenderTarget,
        destination: &BishopRenderTarget,
        params: EmissionPassParams,
    ) -> Result<(), String>;

    /// Applies one point light to a previously encoded illumination target.
    fn apply_point_light_pass(
        &mut self,
        source: &BishopRenderTarget,
        destination: &BishopRenderTarget,
        params: PointLightPassParams,
    ) -> Result<(), String>;

    /// Combines raw albedo with encoded illumination and tone-maps into a distinct destination.
    fn composite_lighting_pass(
        &mut self,
        albedo: &BishopRenderTarget,
        illumination: &BishopRenderTarget,
        destination: &BishopRenderTarget,
    ) -> Result<(), String>;

    /// Returns a creation or backend-ownership failure attached to a target.
    fn render_target_error(&self, rt: &BishopRenderTarget) -> Option<String>;

    /// Creates a render target compatible with the texture renderer for drawing via `draw_render_target`.
    fn create_drawable_render_target(&self, width: u32, height: u32) -> BishopRenderTarget;
}
