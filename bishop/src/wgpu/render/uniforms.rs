//! Uniform buffer types for shaders.

use crate::camera::Camera2D;
use bytemuck::{Pod, Zeroable};

/// Camera uniforms for 2D rendering.
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct CameraUniforms {
    /// Orthographic projection matrix.
    pub projection: [[f32; 4]; 4],
}

impl CameraUniforms {
    /// Creates screen-space projection (origin at top-left, y increases downward).
    pub fn screen_space(width: f32, height: f32) -> Self {
        let projection = glam::Mat4::orthographic_rh(0.0, width, height, 0.0, -1.0, 1.0);
        Self {
            projection: projection.to_cols_array_2d(),
        }
    }

    /// Creates projection from a Camera2D.
    /// The camera zoom convention follows macroquad: zoom = 2.0 / screen_size for 1:1 mapping.
    pub fn from_camera2d(camera: &Camera2D, _width: f32, _height: f32) -> Self {
        let half_w = 1.0 / camera.zoom.x;
        let half_h = 1.0 / camera.zoom.y;

        let left = camera.target.x - half_w + camera.offset.x / camera.zoom.x;
        let right = camera.target.x + half_w + camera.offset.x / camera.zoom.x;
        let top = camera.target.y - half_h + camera.offset.y / camera.zoom.y;
        let bottom = camera.target.y + half_h + camera.offset.y / camera.zoom.y;

        let mut projection = glam::Mat4::orthographic_rh(left, right, bottom, top, -1.0, 1.0);

        if camera.rotation != 0.0 {
            let rotation = glam::Mat4::from_rotation_z(-camera.rotation.to_radians());
            projection *= rotation;
        }

        Self {
            projection: projection.to_cols_array_2d(),
        }
    }
}

impl Default for CameraUniforms {
    fn default() -> Self {
        Self::screen_space(800.0, 600.0)
    }
}

/// Model transform uniforms for vertex shaders.
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct ModelUniforms {
    /// Model transformation matrix.
    pub model: [[f32; 4]; 4],
}

impl Default for ModelUniforms {
    fn default() -> Self {
        Self {
            model: glam::Mat4::IDENTITY.to_cols_array_2d(),
        }
    }
}

/// Ambient shader uniforms for darkness application.
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct AmbientUniforms {
    /// Darkness level (0.0 = full brightness, 1.0 = fully dark).
    pub darkness: f32,
    pub _pad: [f32; 3],
}

impl Default for AmbientUniforms {
    fn default() -> Self {
        Self {
            darkness: 0.0,
            _pad: [0.0; 3],
        }
    }
}

/// Grid shader uniforms for editor grid overlay.
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct GridUniforms {
    /// Camera position in world space.
    pub camera_pos: [f32; 2],
    /// Camera zoom level.
    pub camera_zoom: f32,
    /// Grid cell size in world units.
    pub grid_size: f32,
    /// Viewport size in pixels.
    pub viewport_size: [f32; 2],
    /// Line thickness in pixels.
    pub line_thickness: f32,
    pub _pad: f32,
    /// Grid line color (RGBA).
    pub line_color: [f32; 4],
}

impl Default for GridUniforms {
    fn default() -> Self {
        Self {
            camera_pos: [0.0, 0.0],
            camera_zoom: 1.0,
            grid_size: 32.0,
            viewport_size: [800.0, 600.0],
            line_thickness: 1.0,
            _pad: 0.0,
            line_color: [0.5, 0.5, 0.5, 0.5],
        }
    }
}

/// Single spotlight data for the spotlight shader.
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct SpotLightData {
    /// Light position in screen coordinates.
    pub pos: [f32; 2],
    /// Light intensity for color tinting.
    pub intensity: f32,
    /// Light radius in pixels.
    pub radius: f32,
    /// Light color (RGB).
    pub color: [f32; 3],
    /// Light spread/falloff distance.
    pub spread: f32,
    /// Light alpha/visibility.
    pub alpha: f32,
    /// Additive brightness contribution.
    pub brightness: f32,
    pub _pad: [f32; 2],
}

impl Default for SpotLightData {
    fn default() -> Self {
        Self {
            pos: [0.0, 0.0],
            intensity: 0.5,
            radius: 100.0,
            color: [1.0, 1.0, 1.0],
            spread: 50.0,
            alpha: 1.0,
            brightness: 0.0,
            _pad: [0.0; 2],
        }
    }
}

/// Spotlight shader uniforms with array of lights.
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct SpotUniforms {
    /// Screen size in pixels.
    pub screen_size: [f32; 2],
    /// Darkness level for ambient lighting.
    pub darkness: f32,
    /// Number of active lights.
    pub light_count: i32,
    /// Array of spotlight data (max 10).
    pub lights: [SpotLightData; 10],
}

impl Default for SpotUniforms {
    fn default() -> Self {
        Self {
            screen_size: [800.0, 600.0],
            darkness: 0.0,
            light_count: 0,
            lights: [SpotLightData::default(); 10],
        }
    }
}

/// Single glow source data for the glow shader.
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct GlowData {
    /// Mask position in screen coordinates.
    pub mask_pos: [f32; 2],
    /// Mask size in pixels.
    pub mask_size: [f32; 2],
    /// Glow color (RGB).
    pub color: [f32; 3],
    /// Glow brightness multiplier.
    pub brightness: f32,
    /// Glow intensity for color tinting.
    pub intensity: f32,
    /// Emission level for blur scaling.
    pub emission: f32,
    pub _pad: [f32; 2],
}

impl Default for GlowData {
    fn default() -> Self {
        Self {
            mask_pos: [0.0, 0.0],
            mask_size: [64.0, 64.0],
            color: [1.0, 1.0, 1.0],
            brightness: 1.0,
            intensity: 0.5,
            emission: 0.5,
            _pad: [0.0; 2],
        }
    }
}

/// Glow shader uniforms with array of glow sources.
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct GlowUniforms {
    /// Screen size in pixels.
    pub screen_size: [f32; 2],
    /// Number of active glow sources.
    pub glow_count: i32,
    pub _pad: f32,
    /// Array of glow source data (max 10).
    pub glows: [GlowData; 10],
}

impl Default for GlowUniforms {
    fn default() -> Self {
        Self {
            screen_size: [800.0, 600.0],
            glow_count: 0,
            _pad: 0.0,
            glows: [GlowData::default(); 10],
        }
    }
}
