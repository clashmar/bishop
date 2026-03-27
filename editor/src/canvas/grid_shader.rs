// editor/src/canvas/grid_shader.rs
use bishop::prelude::*;
use bishop::wgpu::{FullscreenQuadRenderer, GridMaterial, GridUniforms, ModelUniforms};

/// Parameters for drawing the shader-based grid.
pub struct GridParams {
    pub camera_pos: Vec2,
    pub camera_zoom: f32,
    pub viewport_size: Vec2,
    pub grid_size: f32,
    pub line_color: Color,
    pub line_thickness: f32,
}

/// Renderer for editor grid overlay using wgpu.
pub struct GridRenderer {
    material: GridMaterial,
    fullscreen_quad: FullscreenQuadRenderer,
}

impl GridRenderer {
    /// Creates a new grid renderer.
    pub fn new(ctx: &WgpuContext) -> Self {
        let device = ctx.device();
        let format = ctx.surface_format();

        let fullscreen_quad = FullscreenQuadRenderer::new(device);
        let material =
            GridMaterial::new(device, format, fullscreen_quad.camera_bind_group_layout());

        Self {
            material,
            fullscreen_quad,
        }
    }

    /// Draws the grid to the current surface.
    pub fn draw(&self, ctx: &mut WgpuContext, params: &GridParams) {
        let queue = ctx.queue();
        let width = ctx.screen_width();
        let height = ctx.screen_height();

        let uniforms = GridUniforms {
            camera_pos: [params.camera_pos.x, params.camera_pos.y],
            camera_zoom: params.camera_zoom,
            grid_size: params.grid_size,
            viewport_size: [params.viewport_size.x, params.viewport_size.y],
            line_thickness: params.line_thickness,
            _pad: 0.0,
            line_color: [
                params.line_color.r,
                params.line_color.g,
                params.line_color.b,
                params.line_color.a,
            ],
        };

        self.material.set_uniforms(queue, &uniforms);
        self.fullscreen_quad.update_camera(queue, width, height);
        self.fullscreen_quad.update_model(
            queue,
            &ModelUniforms {
                model: glam::Mat4::from_scale(glam::Vec3::new(width, height, 1.0))
                    .to_cols_array_2d(),
            },
        );

        ctx.flush_if_needed();

        // Determine load operation - clear if not yet cleared this frame
        let load_op = if ctx.has_cleared_this_frame() {
            wgpu::LoadOp::Load
        } else {
            let clear_color = ctx.clear_color().unwrap_or(Color::BLACK);
            ctx.mark_cleared();
            wgpu::LoadOp::Clear(wgpu::Color {
                r: clear_color.r as f64,
                g: clear_color.g as f64,
                b: clear_color.b as f64,
                a: clear_color.a as f64,
            })
        };

        let device = ctx.device();
        let surface_view = match ctx.current_surface_view() {
            Some(view) => view,
            None => return,
        };

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("grid_encoder"),
        });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("grid_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: surface_view,
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

            render_pass.set_pipeline(self.material.pipeline());
            self.fullscreen_quad.prepare_pass(&mut render_pass);
            render_pass.set_bind_group(1, self.material.uniform_bind_group(), &[]);
            self.fullscreen_quad.draw(&mut render_pass);
        }

        ctx.queue().submit(std::iter::once(encoder.finish()));
    }
}
