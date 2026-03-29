//! Text rendering using fontdue for glyph rasterization.

use std::collections::HashMap;

use wgpu::util::DeviceExt;

use super::sampler::create_nearest_sampler;
use super::uniforms::CameraUniforms;
use super::vertex::TexturedVertex;
use crate::text::TextDimensions;
use crate::types::Color;

const ATLAS_SIZE: u32 = 1024;
const MAX_VERTICES: usize = 65536;

static GNF_FONT_DATA: &[u8] = include_bytes!("../../fonts/gnf.regular.ttf");

/// Key for cached glyphs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct GlyphKey {
    character: char,
    font_size_px: u32,
}

/// Information about a cached glyph in the atlas.
#[derive(Clone, Copy, Debug)]
struct GlyphInfo {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    advance_width: f32,
    offset_x: f32,
    offset_y: f32,
}

/// Font atlas that caches rasterized glyphs on the GPU.
pub struct FontAtlas {
    font: fontdue::Font,
    atlas_data: Vec<u8>,
    atlas_texture: Option<wgpu::Texture>,
    atlas_view: Option<wgpu::TextureView>,
    sampler: Option<wgpu::Sampler>,
    bind_group: Option<wgpu::BindGroup>,
    cache: HashMap<GlyphKey, GlyphInfo>,
    cursor_x: u32,
    cursor_y: u32,
    row_height: u32,
    dirty: bool,
}

impl FontAtlas {
    /// Creates a new font atlas from TTF data.
    pub fn new(ttf_data: &[u8]) -> Result<Self, &'static str> {
        let font = fontdue::Font::from_bytes(ttf_data, fontdue::FontSettings::default())
            .map_err(|_| "Failed to parse font")?;

        let atlas_data = vec![0u8; (ATLAS_SIZE * ATLAS_SIZE * 4) as usize];

        Ok(Self {
            font,
            atlas_data,
            atlas_texture: None,
            atlas_view: None,
            sampler: None,
            bind_group: None,
            cache: HashMap::new(),
            cursor_x: 0,
            cursor_y: 0,
            row_height: 0,
            dirty: true,
        })
    }

    /// Creates a font atlas with the embedded GNF font.
    pub fn with_default_font() -> Result<Self, &'static str> {
        Self::new(GNF_FONT_DATA)
    }

    /// Initializes GPU resources for the atlas.
    pub fn init_gpu(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        texture_bind_group_layout: &wgpu::BindGroupLayout,
    ) {
        let size = wgpu::Extent3d {
            width: ATLAS_SIZE,
            height: ATLAS_SIZE,
            depth_or_array_layers: 1,
        };

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("font_atlas_texture"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let sampler = create_nearest_sampler(device, "font_atlas_sampler");

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("font_atlas_bind_group"),
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

        self.atlas_texture = Some(texture);
        self.atlas_view = Some(view);
        self.sampler = Some(sampler);
        self.bind_group = Some(bind_group);

        self.upload(queue);
    }

    /// Gets or rasterizes a glyph, returning its info.
    fn get_glyph(&mut self, character: char, font_size: f32) -> Option<GlyphInfo> {
        let font_size_px = font_size as u32;
        let key = GlyphKey {
            character,
            font_size_px,
        };

        if let Some(info) = self.cache.get(&key) {
            return Some(*info);
        }

        let (metrics, bitmap) = self.font.rasterize(character, font_size);

        if metrics.width == 0 || metrics.height == 0 {
            let info = GlyphInfo {
                x: 0,
                y: 0,
                width: 0,
                height: 0,
                advance_width: metrics.advance_width,
                offset_x: metrics.xmin as f32,
                offset_y: metrics.ymin as f32,
            };
            self.cache.insert(key, info);
            return Some(info);
        }

        let glyph_w = metrics.width as u32;
        let glyph_h = metrics.height as u32;

        if self.cursor_x + glyph_w >= ATLAS_SIZE {
            self.cursor_x = 0;
            self.cursor_y += self.row_height + 1;
            self.row_height = 0;
        }

        if self.cursor_y + glyph_h >= ATLAS_SIZE {
            return None;
        }

        let gx = self.cursor_x;
        let gy = self.cursor_y;

        for (i, alpha) in bitmap.iter().enumerate() {
            let px = (i % metrics.width) as u32;
            let py = (i / metrics.width) as u32;
            let atlas_x = gx + px;
            let atlas_y = gy + py;
            let idx = ((atlas_y * ATLAS_SIZE + atlas_x) * 4) as usize;
            self.atlas_data[idx] = 255;
            self.atlas_data[idx + 1] = 255;
            self.atlas_data[idx + 2] = 255;
            self.atlas_data[idx + 3] = *alpha;
        }

        let info = GlyphInfo {
            x: gx,
            y: gy,
            width: glyph_w,
            height: glyph_h,
            advance_width: metrics.advance_width,
            offset_x: metrics.xmin as f32,
            offset_y: metrics.ymin as f32,
        };

        self.cache.insert(key, info);
        self.cursor_x += glyph_w + 1;
        self.row_height = self.row_height.max(glyph_h);
        self.dirty = true;

        Some(info)
    }

    /// Uploads the atlas texture to the GPU if dirty.
    pub fn upload(&mut self, queue: &wgpu::Queue) {
        if !self.dirty {
            return;
        }

        if let Some(texture) = &self.atlas_texture {
            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                &self.atlas_data,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(4 * ATLAS_SIZE),
                    rows_per_image: Some(ATLAS_SIZE),
                },
                wgpu::Extent3d {
                    width: ATLAS_SIZE,
                    height: ATLAS_SIZE,
                    depth_or_array_layers: 1,
                },
            );
        }

        self.dirty = false;
    }

    /// Measures text without drawing it.
    pub fn measure_text(&mut self, text: &str, font_size: f32) -> TextDimensions {
        let mut width = 0.0f32;

        for ch in text.chars() {
            if let Some(info) = self.get_glyph(ch, font_size) {
                width += info.advance_width;
            }
        }

        let line_metrics = self.font.horizontal_line_metrics(font_size);
        let (ascent, descent) = match line_metrics {
            Some(m) => (m.ascent, -m.descent),
            None => (font_size, 0.0),
        };

        TextDimensions {
            width,
            height: ascent + descent,
            offset_y: ascent,
        }
    }

    /// Returns the font's global line metrics for the given pixel size.
    pub fn font_line_metrics(&self, font_size: f32) -> Option<fontdue::LineMetrics> {
        self.font.horizontal_line_metrics(font_size)
    }

    /// Pre-caches common characters at multiple sizes.
    pub fn precache(&mut self) {
        let chars: Vec<char> = (32u8..=126).map(|c| c as char).collect();
        let extra_chars = ['⌘', '⌥', '⇧', '↓', '→'];

        for size in [
            12.0, 14.0, 15.0, 16.0, 18.0, 20.0, 24.0, 28.0, 32.0, 36.0, 48.0,
        ] {
            for &ch in &chars {
                self.get_glyph(ch, size);
            }
            for &ch in &extra_chars {
                self.get_glyph(ch, size);
            }
        }
    }

    /// Returns the atlas bind group for rendering.
    pub fn bind_group(&self) -> Option<&wgpu::BindGroup> {
        self.bind_group.as_ref()
    }
}

/// Text renderer using the font atlas.
pub struct TextRenderer {
    pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    uniform_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    #[allow(dead_code)]
    texture_bind_group_layout: wgpu::BindGroupLayout,
    font_atlas: FontAtlas,
    vertices: Vec<TexturedVertex>,
}

impl TextRenderer {
    /// Creates a new text renderer with the default font.
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        surface_format: wgpu::TextureFormat,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("text_shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/textured.wgsl").into()),
        });

        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("text_uniform_buffer"),
            contents: bytemuck::cast_slice(&[CameraUniforms::default()]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let camera_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("text_camera_bind_group_layout"),
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
            label: Some("text_camera_bind_group"),
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("text_texture_bind_group_layout"),
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
            });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("text_pipeline_layout"),
            bind_group_layouts: &[&camera_bind_group_layout, &texture_bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("text_pipeline"),
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
            label: Some("text_vertex_buffer"),
            size: (MAX_VERTICES * std::mem::size_of::<TexturedVertex>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let mut font_atlas = FontAtlas::with_default_font().expect("Failed to create font atlas");
        font_atlas.init_gpu(device, queue, &texture_bind_group_layout);
        font_atlas.precache();
        font_atlas.upload(queue);

        Self {
            pipeline,
            vertex_buffer,
            uniform_buffer,
            camera_bind_group,
            texture_bind_group_layout,
            font_atlas,
            vertices: Vec::with_capacity(MAX_VERTICES),
        }
    }

    /// Returns the texture bind group layout for creating custom textures.
    #[allow(dead_code)]
    pub fn texture_bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.texture_bind_group_layout
    }

    /// Clears all queued text for a new frame.
    pub fn clear(&mut self) {
        self.vertices.clear();
    }

    /// Returns the number of text vertices queued.
    pub fn vertex_count(&self) -> usize {
        self.vertices.len()
    }

    /// Returns true if there are no queued text draws.
    pub fn is_empty(&self) -> bool {
        self.vertices.is_empty()
    }

    /// Updates the camera uniform buffer.
    pub fn update_uniforms(&self, queue: &wgpu::Queue, uniforms: &CameraUniforms) {
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[*uniforms]));
    }

    /// Draws text at the specified position and returns its dimensions.
    pub fn draw_text(
        &mut self,
        text: &str,
        x: f32,
        y: f32,
        font_size: f32,
        color: Color,
    ) -> TextDimensions {
        let c: [f32; 4] = color.into();
        let mut cursor_x = x;
        let baseline_y = y;

        for ch in text.chars() {
            if let Some(info) = self.font_atlas.get_glyph(ch, font_size) {
                if info.width > 0 && info.height > 0 {
                    let gx = cursor_x + info.offset_x;
                    let gy = baseline_y - info.height as f32 - info.offset_y;

                    let u0 = info.x as f32 / ATLAS_SIZE as f32;
                    let v0 = info.y as f32 / ATLAS_SIZE as f32;
                    let u1 = (info.x + info.width) as f32 / ATLAS_SIZE as f32;
                    let v1 = (info.y + info.height) as f32 / ATLAS_SIZE as f32;

                    let gw = info.width as f32;
                    let gh = info.height as f32;

                    let v0_vert = TexturedVertex::new([gx, gy], [u0, v0], c);
                    let v1_vert = TexturedVertex::new([gx + gw, gy], [u1, v0], c);
                    let v2_vert = TexturedVertex::new([gx + gw, gy + gh], [u1, v1], c);
                    let v3_vert = TexturedVertex::new([gx, gy + gh], [u0, v1], c);

                    self.vertices
                        .extend_from_slice(&[v0_vert, v1_vert, v2_vert, v0_vert, v2_vert, v3_vert]);
                }

                cursor_x += info.advance_width;
            }
        }

        let line_metrics = self.font_atlas.font_line_metrics(font_size);
        let (ascent, descent) = match line_metrics {
            Some(m) => (m.ascent, -m.descent),
            None => (font_size, 0.0),
        };

        TextDimensions {
            width: cursor_x - x,
            height: ascent + descent,
            offset_y: ascent,
        }
    }

    /// Measures text without drawing it.
    #[allow(dead_code)]
    pub fn measure_text(&mut self, text: &str, font_size: f32) -> TextDimensions {
        self.font_atlas.measure_text(text, font_size)
    }

    /// Draws text with extended parameters including rotation support.
    pub fn draw_text_ex(
        &mut self,
        text: &str,
        x: f32,
        y: f32,
        params: &crate::text::TextParams,
    ) -> TextDimensions {
        let effective_font_size = params.font_size as f32 * params.font_scale;

        if params.rotation == 0.0 {
            return self.draw_text(text, x, y, effective_font_size, params.color);
        }

        let c: [f32; 4] = params.color.into();
        let font_scale_x = params.font_scale * params.font_scale_aspect;
        let font_scale_y = params.font_scale;

        let dims = self.font_atlas.measure_text(text, params.font_size as f32);
        let scaled_width = dims.width * font_scale_x;
        let scaled_height = dims.height * font_scale_y;
        let scaled_ascent = dims.offset_y * font_scale_y;

        let pivot_x = scaled_width * 0.5;
        let pivot_y = scaled_height * 0.5;

        let center_x = x + pivot_x;
        let center_y = y - scaled_ascent + pivot_y;

        let cos_r = params.rotation.cos();
        let sin_r = params.rotation.sin();

        let mut cursor_x = 0.0f32;

        for ch in text.chars() {
            if let Some(info) = self.font_atlas.get_glyph(ch, params.font_size as f32) {
                if info.width > 0 && info.height > 0 {
                    let local_x = cursor_x * font_scale_x + info.offset_x * font_scale_x;
                    let local_y = dims.offset_y * font_scale_y
                        - info.height as f32 * font_scale_y
                        - info.offset_y * font_scale_y;

                    let gw = info.width as f32 * font_scale_x;
                    let gh = info.height as f32 * font_scale_y;

                    let u0 = info.x as f32 / ATLAS_SIZE as f32;
                    let v0 = info.y as f32 / ATLAS_SIZE as f32;
                    let u1 = (info.x + info.width) as f32 / ATLAS_SIZE as f32;
                    let v1 = (info.y + info.height) as f32 / ATLAS_SIZE as f32;

                    let corners = [
                        [local_x, local_y],
                        [local_x + gw, local_y],
                        [local_x + gw, local_y + gh],
                        [local_x, local_y + gh],
                    ];

                    let mut rotated = [[0.0f32; 2]; 4];
                    for (i, corner) in corners.iter().enumerate() {
                        let dx = corner[0] - pivot_x;
                        let dy = corner[1] - pivot_y;
                        rotated[i] = [
                            center_x + dx * cos_r - dy * sin_r,
                            center_y + dx * sin_r + dy * cos_r,
                        ];
                    }

                    let v0_vert = TexturedVertex::new(rotated[0], [u0, v0], c);
                    let v1_vert = TexturedVertex::new(rotated[1], [u1, v0], c);
                    let v2_vert = TexturedVertex::new(rotated[2], [u1, v1], c);
                    let v3_vert = TexturedVertex::new(rotated[3], [u0, v1], c);

                    self.vertices
                        .extend_from_slice(&[v0_vert, v1_vert, v2_vert, v0_vert, v2_vert, v3_vert]);
                }

                cursor_x += info.advance_width;
            }
        }

        TextDimensions {
            width: scaled_width,
            height: scaled_height,
            offset_y: dims.offset_y * font_scale_y,
        }
    }

    /// Uploads any dirty atlas pixel data to the GPU. Must be called before the render pass.
    pub fn upload_atlas(&mut self, queue: &wgpu::Queue) {
        self.font_atlas.upload(queue);
    }

    /// Uploads the vertex buffer to the GPU.
    pub fn upload_vertices(&self, queue: &wgpu::Queue) {
        if self.vertices.is_empty() {
            return;
        }
        queue.write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&self.vertices));
    }

    /// Binds the pipeline, camera group, atlas group, and vertex buffer.
    /// Returns false if the atlas bind group is not yet ready.
    pub fn setup_pipeline<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) -> bool {
        let Some(bind_group) = self.font_atlas.bind_group() else {
            return false;
        };
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
        render_pass.set_bind_group(1, bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        true
    }

    /// Issues a draw call for a sub-range of the uploaded vertex buffer.
    pub fn draw_range(&self, render_pass: &mut wgpu::RenderPass<'_>, start: u32, count: u32) {
        render_pass.draw(start..start + count, 0..1);
    }
}
