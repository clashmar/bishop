//! Texture loading and rendering for wgpu backend.

use wgpu::util::DeviceExt;

use super::sampler::create_nearest_sampler;
use super::uniforms::CameraUniforms;
use super::vertex::TexturedVertex;
use crate::draw::DrawTextureParams;
use crate::types::Color;

const MAX_VERTICES: usize = 65536;

/// A GPU texture with associated sampler and bind group.
pub struct WgpuTexture {
    #[allow(dead_code)]
    texture: wgpu::Texture,
    #[allow(dead_code)]
    view: wgpu::TextureView,
    #[allow(dead_code)]
    sampler: wgpu::Sampler,
    bind_group: wgpu::BindGroup,
    width: u32,
    height: u32,
}

impl WgpuTexture {
    /// Creates a texture from RGBA pixel data.
    pub fn from_rgba(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        texture_bind_group_layout: &wgpu::BindGroupLayout,
        data: &[u8],
        width: u32,
        height: u32,
    ) -> Self {
        let size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("wgpu_texture"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * width),
                rows_per_image: Some(height),
            },
            size,
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let sampler = create_nearest_sampler(device, "texture_sampler");

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("texture_bind_group"),
            layout: texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });

        Self {
            texture,
            view,
            sampler,
            bind_group,
            width,
            height,
        }
    }

    /// Creates a texture from PNG image data.
    pub fn from_png(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        texture_bind_group_layout: &wgpu::BindGroupLayout,
        data: &[u8],
    ) -> Result<Self, image::ImageError> {
        let img = image::load_from_memory(data)?;
        let rgba = img.to_rgba8();
        let (width, height) = rgba.dimensions();
        Ok(Self::from_rgba(
            device,
            queue,
            texture_bind_group_layout,
            &rgba,
            width,
            height,
        ))
    }

    /// Returns the texture dimensions.
    pub fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// Returns the texture width.
    pub fn width(&self) -> u32 {
        self.width
    }

    /// Returns the texture height.
    pub fn height(&self) -> u32 {
        self.height
    }
}

/// Batched renderer for textured quads.
pub struct TextureRenderer {
    pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    uniform_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    texture_bind_group_layout: std::sync::Arc<wgpu::BindGroupLayout>,
    vertices: Vec<TexturedVertex>,
    current_texture_bind_group: Option<usize>,
    batches: Vec<TextureBatch>,
}

struct TextureBatch {
    bind_group_ptr: *const wgpu::BindGroup,
    start_vertex: u32,
    vertex_count: u32,
}

unsafe impl Send for TextureBatch {}
unsafe impl Sync for TextureBatch {}

impl TextureRenderer {
    /// Creates a new texture renderer.
    pub fn new(device: &wgpu::Device, surface_format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("textured_shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/textured.wgsl").into()),
        });

        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("texture_uniform_buffer"),
            contents: bytemuck::cast_slice(&[CameraUniforms::default()]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let camera_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("texture_camera_bind_group_layout"),
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

        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("texture_camera_bind_group"),
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        let texture_bind_group_layout = std::sync::Arc::new(device.create_bind_group_layout(
            &wgpu::BindGroupLayoutDescriptor {
                label: Some("texture_bind_group_layout"),
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
                ],
            },
        ));

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("texture_pipeline_layout"),
            bind_group_layouts: &[&camera_bind_group_layout, &texture_bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("texture_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[TexturedVertex::layout()],
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
            label: Some("texture_vertex_buffer"),
            size: (MAX_VERTICES * std::mem::size_of::<TexturedVertex>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            pipeline,
            vertex_buffer,
            uniform_buffer,
            camera_bind_group,
            texture_bind_group_layout,
            vertices: Vec::with_capacity(MAX_VERTICES),
            current_texture_bind_group: None,
            batches: Vec::new(),
        }
    }

    /// Returns a reference to the texture bind group layout for creating textures.
    pub fn texture_bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.texture_bind_group_layout
    }

    /// Returns an Arc clone of the texture bind group layout for shared ownership.
    pub fn texture_bind_group_layout_arc(&self) -> std::sync::Arc<wgpu::BindGroupLayout> {
        self.texture_bind_group_layout.clone()
    }

    /// Clears all queued draws for a new frame.
    pub fn clear(&mut self) {
        self.vertices.clear();
        self.batches.clear();
        self.current_texture_bind_group = None;
    }

    /// Updates the camera uniform buffer.
    pub fn update_uniforms(&self, queue: &wgpu::Queue, uniforms: &CameraUniforms) {
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[*uniforms]));
    }

    /// Returns true if there are no queued draws.
    pub fn is_empty(&self) -> bool {
        self.vertices.is_empty() && self.batches.is_empty()
    }

    /// Returns the number of texture batches queued.
    pub fn batch_count(&self) -> usize {
        self.batches.len()
    }

    /// Forces the next texture draw to start a new batch, regardless of bind group.
    pub fn seal_batch(&mut self) {
        self.current_texture_bind_group = None;
    }

    /// Draws a texture at the specified position.
    pub fn draw_texture(&mut self, texture: &WgpuTexture, x: f32, y: f32, color: Color) {
        self.draw_texture_ex(texture, x, y, color, DrawTextureParams::default());
    }

    /// Draws a texture with extended parameters.
    pub fn draw_texture_ex(
        &mut self,
        texture: &WgpuTexture,
        x: f32,
        y: f32,
        color: Color,
        params: DrawTextureParams,
    ) {
        let tex_w = texture.width() as f32;
        let tex_h = texture.height() as f32;

        let (src_x, src_y, src_w, src_h) = if let Some(source) = params.source {
            (source.x, source.y, source.w, source.h)
        } else {
            (0.0, 0.0, tex_w, tex_h)
        };

        let (dest_w, dest_h) = if let Some(dest_size) = params.dest_size {
            (dest_size.x, dest_size.y)
        } else {
            (src_w, src_h)
        };

        let mut u0 = src_x / tex_w;
        let mut u1 = (src_x + src_w) / tex_w;
        let mut v0 = src_y / tex_h;
        let mut v1 = (src_y + src_h) / tex_h;

        if params.flip_x {
            std::mem::swap(&mut u0, &mut u1);
        }
        if params.flip_y {
            std::mem::swap(&mut v0, &mut v1);
        }

        let c: [f32; 4] = color.into();

        let bind_group_ptr = &texture.bind_group as *const wgpu::BindGroup;
        let bind_group_id = bind_group_ptr as usize;

        let needs_new_batch = self.current_texture_bind_group != Some(bind_group_id);

        if needs_new_batch {
            self.batches.push(TextureBatch {
                bind_group_ptr,
                start_vertex: self.vertices.len() as u32,
                vertex_count: 0,
            });
            self.current_texture_bind_group = Some(bind_group_id);
        }

        if params.rotation != 0.0 {
            let pivot = params
                .pivot
                .unwrap_or(glam::Vec2::new(dest_w * 0.5, dest_h * 0.5));
            let cos_r = params.rotation.cos();
            let sin_r = params.rotation.sin();

            let corners = [
                glam::Vec2::new(0.0, 0.0),
                glam::Vec2::new(dest_w, 0.0),
                glam::Vec2::new(dest_w, dest_h),
                glam::Vec2::new(0.0, dest_h),
            ];

            let mut rotated = [[0.0f32; 2]; 4];
            for (i, corner) in corners.iter().enumerate() {
                let dx = corner.x - pivot.x;
                let dy = corner.y - pivot.y;
                rotated[i] = [
                    x + pivot.x + dx * cos_r - dy * sin_r,
                    y + pivot.y + dx * sin_r + dy * cos_r,
                ];
            }

            let v0_vert = TexturedVertex::new(rotated[0], [u0, v0], c);
            let v1_vert = TexturedVertex::new(rotated[1], [u1, v0], c);
            let v2_vert = TexturedVertex::new(rotated[2], [u1, v1], c);
            let v3_vert = TexturedVertex::new(rotated[3], [u0, v1], c);

            self.vertices
                .extend_from_slice(&[v0_vert, v1_vert, v2_vert, v0_vert, v2_vert, v3_vert]);
        } else {
            let v0_vert = TexturedVertex::new([x, y], [u0, v0], c);
            let v1_vert = TexturedVertex::new([x + dest_w, y], [u1, v0], c);
            let v2_vert = TexturedVertex::new([x + dest_w, y + dest_h], [u1, v1], c);
            let v3_vert = TexturedVertex::new([x, y + dest_h], [u0, v1], c);

            self.vertices
                .extend_from_slice(&[v0_vert, v1_vert, v2_vert, v0_vert, v2_vert, v3_vert]);
        }

        if let Some(batch) = self.batches.last_mut() {
            batch.vertex_count += 6;
        }
    }

    /// Draws a render target as a textured quad using its bind group directly.
    pub fn draw_render_target_quad(
        &mut self,
        bind_group: &wgpu::BindGroup,
        x: f32,
        y: f32,
        dest_w: f32,
        dest_h: f32,
    ) {
        let bind_group_ptr = bind_group as *const wgpu::BindGroup;
        let bind_group_id = bind_group_ptr as usize;

        let needs_new_batch = self.current_texture_bind_group != Some(bind_group_id);
        if needs_new_batch {
            self.batches.push(TextureBatch {
                bind_group_ptr,
                start_vertex: self.vertices.len() as u32,
                vertex_count: 0,
            });
            self.current_texture_bind_group = Some(bind_group_id);
        }

        let c: [f32; 4] = Color::WHITE.into();

        let v0 = TexturedVertex::new([x, y], [0.0, 0.0], c);
        let v1 = TexturedVertex::new([x + dest_w, y], [1.0, 0.0], c);
        let v2 = TexturedVertex::new([x + dest_w, y + dest_h], [1.0, 1.0], c);
        let v3 = TexturedVertex::new([x, y + dest_h], [0.0, 1.0], c);

        self.vertices.extend_from_slice(&[v0, v1, v2, v0, v2, v3]);

        if let Some(batch) = self.batches.last_mut() {
            batch.vertex_count += 6;
        }
    }

    /// Uploads the vertex buffer to the GPU.
    pub fn upload_vertices(&self, queue: &wgpu::Queue) {
        if self.vertices.is_empty() {
            return;
        }
        queue.write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&self.vertices));
    }

    /// Binds the pipeline, camera group, and vertex buffer on the render pass.
    pub fn setup_pipeline<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
    }

    /// Issues draw calls for a sub-range of the batch list.
    pub fn draw_batches_range<'a>(
        &'a self,
        render_pass: &mut wgpu::RenderPass<'a>,
        batch_start: usize,
        batch_count: usize,
    ) {
        for batch in &self.batches[batch_start..batch_start + batch_count] {
            let bind_group = unsafe { &*batch.bind_group_ptr };
            render_pass.set_bind_group(1, bind_group, &[]);
            render_pass.draw(
                batch.start_vertex..batch.start_vertex + batch.vertex_count,
                0..1,
            );
        }
    }
}
