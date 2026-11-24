//! Fixed fullscreen lighting passes for the WGPU backend.

use bytemuck::{Pod, Zeroable};
use super::fullscreen_quad::FullscreenQuadRenderer;
use super::render_target::BishopRenderTarget;
use crate::material::{AmbientPassParams, PointLightPassParams};

const FULLSCREEN_QUAD_CAMERA_SIZE: (f32, f32) = (1.0, 1.0);

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct AmbientUniforms {
    color: [f32; 4],
    settings: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct PointLightUniforms {
    center_radius_intensity: [f32; 4],
    color: [f32; 4],
    settings: [f32; 4],
    resolution: [f32; 4],
    pixel_grid_origin: [f32; 4],
}

struct PassPipeline {
    pipeline: wgpu::RenderPipeline,
    uniform_buffer: wgpu::Buffer,
    uniform_bind_group: wgpu::BindGroup,
    texture_bind_group_layout: wgpu::BindGroupLayout,
}

impl PassPipeline {
    fn new(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        camera_layout: &wgpu::BindGroupLayout,
        uniform_size: u64,
        label: &'static str,
        fragment_source: String,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(label),
            source: wgpu::ShaderSource::Wgsl(fragment_source.into()),
        });
        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(label),
            size: uniform_size,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let uniform_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some(label),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(label),
            layout: &uniform_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });
        let texture_bind_group_layout = device.create_bind_group_layout(
            &wgpu::BindGroupLayoutDescriptor {
                label: Some(label),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            },
        );
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some(label),
            bind_group_layouts: &[camera_layout, &uniform_layout, &texture_bind_group_layout],
            push_constant_ranges: &[],
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some(label),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[FullscreenQuadRenderer::vertex_layout()],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: None,
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

        Self {
            pipeline,
            uniform_buffer,
            uniform_bind_group,
            texture_bind_group_layout,
        }
    }

    fn create_texture_bind_group(
        &self,
        device: &wgpu::Device,
        source: &BishopRenderTarget,
        secondary_source: &BishopRenderTarget,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("lighting_pass_source"),
            layout: &self.texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(source.sample_view()),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(source.sampler()),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(secondary_source.sample_view()),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(secondary_source.sampler()),
                },
            ],
        })
    }
}

pub(crate) struct LightingRenderer {
    ambient: PassPipeline,
    point: PassPipeline,
    composite: PassPipeline,
}

impl LightingRenderer {
    pub(crate) fn new(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        quad: &FullscreenQuadRenderer,
    ) -> Self {
        let vertex = include_str!("shaders/vertex.wgsl");
        let ambient = PassPipeline::new(
            device,
            format,
            quad.camera_bind_group_layout(),
            std::mem::size_of::<AmbientUniforms>() as u64,
            "room_ambient_pass",
            format!("{vertex}\n{}", include_str!("shaders/room_ambient.wgsl")),
        );
        let point = PassPipeline::new(
            device,
            format,
            quad.camera_bind_group_layout(),
            std::mem::size_of::<PointLightUniforms>() as u64,
            "point_light_pass",
            format!("{vertex}\n{}", include_str!("shaders/point_light.wgsl")),
        );
        let composite = PassPipeline::new(
            device,
            format,
            quad.camera_bind_group_layout(),
            std::mem::size_of::<AmbientUniforms>() as u64,
            "room_lighting_composite_pass",
            format!("{vertex}\n{}", include_str!("shaders/room_lighting_composite.wgsl")),
        );
        Self { ambient, point, composite }
    }

    pub(crate) fn ambient(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        quad: &FullscreenQuadRenderer,
        source: &BishopRenderTarget,
        destination: &BishopRenderTarget,
        params: AmbientPassParams,
    ) {
        let uniforms = AmbientUniforms {
            color: [params.color[0], params.color[1], params.color[2], 1.0],
            settings: [params.darkness.clamp(0.0, 1.0), params.exposure.max(0.0), 0.0, 0.0],
        };
        queue.write_buffer(&self.ambient.uniform_buffer, 0, bytemuck::bytes_of(&uniforms));
        self.draw(device, queue, encoder, quad, &self.ambient, source, source, destination);
    }

    pub(crate) fn point(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        quad: &FullscreenQuadRenderer,
        source: &BishopRenderTarget,
        destination: &BishopRenderTarget,
        params: PointLightPassParams,
    ) {
        let uniforms = PointLightUniforms {
            center_radius_intensity: [
                params.center[0],
                params.center[1],
                params.radius.max(0.0),
                params.intensity.max(0.0),
            ],
            color: [params.color[0], params.color[1], params.color[2], 1.0],
            settings: [
                params.attenuation.max(0.0),
                params.pixel_size.max(1.0),
                if params.pixelated { 1.0 } else { 0.0 },
                0.0,
            ],
            resolution: [destination.width() as f32, destination.height() as f32, 0.0, 0.0],
            pixel_grid_origin: [
                params.pixel_grid_origin[0],
                params.pixel_grid_origin[1],
                0.0,
                0.0,
            ],
        };
        queue.write_buffer(&self.point.uniform_buffer, 0, bytemuck::bytes_of(&uniforms));
        self.draw(device, queue, encoder, quad, &self.point, source, source, destination);
    }

    pub(crate) fn composite(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        quad: &FullscreenQuadRenderer,
        albedo: &BishopRenderTarget,
        illumination: &BishopRenderTarget,
        destination: &BishopRenderTarget,
    ) {
        self.draw(
            device,
            queue,
            encoder,
            quad,
            &self.composite,
            albedo,
            illumination,
            destination,
        );
    }

    fn draw(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        quad: &FullscreenQuadRenderer,
        pipeline: &PassPipeline,
        source: &BishopRenderTarget,
        secondary_source: &BishopRenderTarget,
        destination: &BishopRenderTarget,
    ) {
        let texture_group = pipeline.create_texture_bind_group(device, source, secondary_source);
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("lighting_pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: destination.render_view(),
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        quad.update_camera(
            queue,
            FULLSCREEN_QUAD_CAMERA_SIZE.0,
            FULLSCREEN_QUAD_CAMERA_SIZE.1,
        );
        pass.set_pipeline(&pipeline.pipeline);
        quad.prepare_pass(&mut pass);
        pass.set_bind_group(1, &pipeline.uniform_bind_group, &[]);
        pass.set_bind_group(2, &texture_group, &[]);
        quad.draw(&mut pass);
    }
}

#[cfg(test)]
mod tests {
    use super::FULLSCREEN_QUAD_CAMERA_SIZE;

    #[test]
    fn normalized_quad_camera_maps_its_unit_square_across_clip_space() {
        let projection = glam::Mat4::orthographic_rh(
            0.0,
            FULLSCREEN_QUAD_CAMERA_SIZE.0,
            FULLSCREEN_QUAD_CAMERA_SIZE.1,
            0.0,
            -1.0,
            1.0,
        );
        let top_left = projection.project_point3(glam::Vec3::ZERO);
        let bottom_right = projection.project_point3(glam::Vec3::ONE);

        assert_eq!((top_left.x, top_left.y), (-1.0, 1.0));
        assert_eq!((bottom_right.x, bottom_right.y), (1.0, -1.0));
    }

    #[test]
    fn lighting_shader_sources_define_shared_vertex_output_once() {
        let vertex = include_str!("shaders/vertex.wgsl");
        for fragment in [
            include_str!("shaders/room_ambient.wgsl"),
            include_str!("shaders/point_light.wgsl"),
            include_str!("shaders/room_lighting_composite.wgsl"),
        ] {
            let source = format!("{vertex}\n{fragment}");
            assert_eq!(source.matches("struct VertexOutput {").count(), 1);
        }
    }

    #[test]
    fn pixel_light_quantizes_falloff_in_target_pixels_without_quantizing_scene_sampling() {
        let wgsl = include_str!("shaders/point_light.wgsl");
        let webgl = include_str!("../../web/lighting.rs");

        assert!(wgsl.contains("let accumulated = textureSample(t_scene, s_scene, in.uv);"));
        assert!(wgsl.contains("var sample_position = in.position.xy;"));
        assert!(!wgsl.contains("in.uv * resolution"));
        assert!(wgsl.contains("let grid_position = sample_position - params.pixel_grid_origin.xy;"));
        assert!(wgsl.contains("let pixel = floor(grid_position / pixel_size)"));
        assert!(webgl.contains("vec4 accumulated = texture(u_scene, v_uv);"));
        assert!(webgl.contains("vec2 grid_position = pixel_position - u_pixel_grid_origin;"));
        assert!(webgl.contains("vec2 pixel = floor(grid_position / pixel_size)"));
    }
}
