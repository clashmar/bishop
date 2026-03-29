//! Renderer for colored 2D primitives.

use std::f32::consts::PI;
use wgpu::util::DeviceExt;

use super::uniforms::CameraUniforms;
use super::vertex::PrimitiveVertex;
use crate::types::Color;

const MAX_VERTICES: usize = 65536;
const CIRCLE_SEGMENTS: usize = 32;

/// Batched renderer for 2D primitives (rectangles, circles, lines, triangles).
pub struct PrimitiveRenderer {
    pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    uniform_buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    vertices: Vec<PrimitiveVertex>,
}

impl PrimitiveRenderer {
    /// Creates a new primitive renderer.
    pub fn new(device: &wgpu::Device, surface_format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("primitive_shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/primitive.wgsl").into()),
        });

        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("primitive_uniform_buffer"),
            contents: bytemuck::cast_slice(&[CameraUniforms::default()]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("primitive_bind_group_layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("primitive_bind_group"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("primitive_pipeline_layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("primitive_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[PrimitiveVertex::layout()],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("primitive_vertex_buffer"),
            size: (MAX_VERTICES * std::mem::size_of::<PrimitiveVertex>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            pipeline,
            vertex_buffer,
            uniform_buffer,
            bind_group,
            vertices: Vec::with_capacity(MAX_VERTICES),
        }
    }

    /// Clears all queued vertices for a new frame.
    pub fn clear(&mut self) {
        self.vertices.clear();
    }

    /// Updates the camera uniform buffer.
    pub fn update_uniforms(&self, queue: &wgpu::Queue, uniforms: &CameraUniforms) {
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[*uniforms]));
    }

    /// Draws a filled rectangle.
    pub fn draw_rectangle(&mut self, x: f32, y: f32, w: f32, h: f32, color: Color) {
        let c: [f32; 4] = color.into();
        let v0 = PrimitiveVertex::new([x, y], c);
        let v1 = PrimitiveVertex::new([x + w, y], c);
        let v2 = PrimitiveVertex::new([x + w, y + h], c);
        let v3 = PrimitiveVertex::new([x, y + h], c);

        self.vertices.extend_from_slice(&[v0, v1, v2, v0, v2, v3]);
    }

    /// Draws a rectangle outline with the specified thickness.
    pub fn draw_rectangle_lines(
        &mut self,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        thickness: f32,
        color: Color,
    ) {
        let half = thickness / 2.0;
        let c: [f32; 4] = color.into();

        // Outer rectangle corners (clockwise from top-left)
        let o0 = PrimitiveVertex::new([x - half, y - half], c);
        let o1 = PrimitiveVertex::new([x + w + half, y - half], c);
        let o2 = PrimitiveVertex::new([x + w + half, y + h + half], c);
        let o3 = PrimitiveVertex::new([x - half, y + h + half], c);

        // Inner rectangle corners (clockwise from top-left)
        let i0 = PrimitiveVertex::new([x + half, y + half], c);
        let i1 = PrimitiveVertex::new([x + w - half, y + half], c);
        let i2 = PrimitiveVertex::new([x + w - half, y + h - half], c);
        let i3 = PrimitiveVertex::new([x + half, y + h - half], c);

        // 8 triangles forming the hollow ring (2 per edge)
        self.vertices.extend_from_slice(&[
            // Top edge
            o0, o1, i1, o0, i1, i0, // Right edge
            o1, o2, i2, o1, i2, i1, // Bottom edge
            o2, o3, i3, o2, i3, i2, // Left edge
            o3, o0, i0, o3, i0, i3,
        ]);
    }

    /// Draws a line between two points with the specified thickness.
    pub fn draw_line(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, thickness: f32, color: Color) {
        let dx = x2 - x1;
        let dy = y2 - y1;
        let len = (dx * dx + dy * dy).sqrt();

        if len < 0.0001 {
            return;
        }

        let nx = -dy / len * thickness * 0.5;
        let ny = dx / len * thickness * 0.5;

        let c: [f32; 4] = color.into();
        let v0 = PrimitiveVertex::new([x1 + nx, y1 + ny], c);
        let v1 = PrimitiveVertex::new([x1 - nx, y1 - ny], c);
        let v2 = PrimitiveVertex::new([x2 - nx, y2 - ny], c);
        let v3 = PrimitiveVertex::new([x2 + nx, y2 + ny], c);

        self.vertices.extend_from_slice(&[v0, v1, v2, v0, v2, v3]);
    }

    /// Draws a filled circle.
    pub fn draw_circle(&mut self, cx: f32, cy: f32, radius: f32, color: Color) {
        let c: [f32; 4] = color.into();
        let center = PrimitiveVertex::new([cx, cy], c);

        for i in 0..CIRCLE_SEGMENTS {
            let angle1 = (i as f32) * 2.0 * PI / CIRCLE_SEGMENTS as f32;
            let angle2 = ((i + 1) as f32) * 2.0 * PI / CIRCLE_SEGMENTS as f32;

            let v1 =
                PrimitiveVertex::new([cx + angle1.cos() * radius, cy + angle1.sin() * radius], c);
            let v2 =
                PrimitiveVertex::new([cx + angle2.cos() * radius, cy + angle2.sin() * radius], c);

            self.vertices.extend_from_slice(&[center, v1, v2]);
        }
    }

    /// Draws a circle outline with the specified thickness.
    pub fn draw_circle_lines(
        &mut self,
        cx: f32,
        cy: f32,
        radius: f32,
        thickness: f32,
        color: Color,
    ) {
        let c: [f32; 4] = color.into();
        let inner_r = radius - thickness * 0.5;
        let outer_r = radius + thickness * 0.5;

        for i in 0..CIRCLE_SEGMENTS {
            let angle1 = (i as f32) * 2.0 * PI / CIRCLE_SEGMENTS as f32;
            let angle2 = ((i + 1) as f32) * 2.0 * PI / CIRCLE_SEGMENTS as f32;

            let cos1 = angle1.cos();
            let sin1 = angle1.sin();
            let cos2 = angle2.cos();
            let sin2 = angle2.sin();

            let inner1 = PrimitiveVertex::new([cx + cos1 * inner_r, cy + sin1 * inner_r], c);
            let outer1 = PrimitiveVertex::new([cx + cos1 * outer_r, cy + sin1 * outer_r], c);
            let inner2 = PrimitiveVertex::new([cx + cos2 * inner_r, cy + sin2 * inner_r], c);
            let outer2 = PrimitiveVertex::new([cx + cos2 * outer_r, cy + sin2 * outer_r], c);

            self.vertices
                .extend_from_slice(&[inner1, outer1, outer2, inner1, outer2, inner2]);
        }
    }

    /// Draws a filled triangle.
    pub fn draw_triangle(&mut self, v1: glam::Vec2, v2: glam::Vec2, v3: glam::Vec2, color: Color) {
        let c: [f32; 4] = color.into();
        let p1 = PrimitiveVertex::new([v1.x, v1.y], c);
        let p2 = PrimitiveVertex::new([v2.x, v2.y], c);
        let p3 = PrimitiveVertex::new([v3.x, v3.y], c);

        self.vertices.extend_from_slice(&[p1, p2, p3]);
    }

    /// Returns the number of vertices queued.
    pub fn vertex_count(&self) -> usize {
        self.vertices.len()
    }

    /// Returns true if there are no queued vertices.
    pub fn is_empty(&self) -> bool {
        self.vertices.is_empty()
    }

    /// Uploads the vertex buffer to the GPU.
    pub fn upload_vertices(&self, queue: &wgpu::Queue) {
        if self.vertices.is_empty() {
            return;
        }
        queue.write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&self.vertices));
    }

    /// Binds the pipeline, uniform group, and vertex buffer on the render pass.
    pub fn setup_pipeline<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
    }

    /// Issues a draw call for a sub-range of the uploaded vertex buffer.
    pub fn draw_range(&self, render_pass: &mut wgpu::RenderPass<'_>, start: u32, count: u32) {
        render_pass.draw(start..start + count, 0..1);
    }
}
