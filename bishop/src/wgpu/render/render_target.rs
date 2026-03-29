//! Render target for off-screen rendering to texture.

use super::sampler::create_nearest_sampler;

/// A render target for off-screen rendering.
/// Wraps a wgpu texture with both render attachment and texture binding capabilities.
#[derive(Debug, Clone)]
pub struct BishopRenderTarget {
    texture: wgpu::Texture,
    render_view: wgpu::TextureView,
    sample_view: wgpu::TextureView,
    sampler: wgpu::Sampler,
    bind_group: wgpu::BindGroup,
    bind_group_layout: std::sync::Arc<wgpu::BindGroupLayout>,
    width: u32,
    height: u32,
    format: wgpu::TextureFormat,
}

impl BishopRenderTarget {
    /// Creates a new render target with the specified dimensions.
    pub fn new(
        device: &wgpu::Device,
        bind_group_layout: std::sync::Arc<wgpu::BindGroupLayout>,
        width: u32,
        height: u32,
        format: wgpu::TextureFormat,
    ) -> Self {
        let (texture, render_view, sample_view, sampler, bind_group) =
            Self::create_resources(device, &bind_group_layout, width, height, format);

        Self {
            texture,
            render_view,
            sample_view,
            sampler,
            bind_group,
            bind_group_layout,
            width,
            height,
            format,
        }
    }

    /// Returns the texture view for use as a render pass color attachment.
    pub fn render_view(&self) -> &wgpu::TextureView {
        &self.render_view
    }

    /// Returns the texture view for shader sampling.
    pub fn sample_view(&self) -> &wgpu::TextureView {
        &self.sample_view
    }

    /// Returns the sampler for shader sampling.
    pub fn sampler(&self) -> &wgpu::Sampler {
        &self.sampler
    }

    /// Returns the bind group for use in shader texture binding.
    pub fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }

    /// Returns a reference to the underlying texture.
    pub fn texture(&self) -> &wgpu::Texture {
        &self.texture
    }

    /// Returns the render target width.
    pub fn width(&self) -> u32 {
        self.width
    }

    /// Returns the render target height.
    pub fn height(&self) -> u32 {
        self.height
    }

    /// Returns the texture format.
    pub fn format(&self) -> wgpu::TextureFormat {
        self.format
    }

    /// Resizes the render target to new dimensions.
    /// Recreates all GPU resources.
    pub fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        if width == self.width && height == self.height {
            return;
        }

        let (texture, render_view, sample_view, sampler, bind_group) =
            Self::create_resources(device, &self.bind_group_layout, width, height, self.format);

        self.texture = texture;
        self.render_view = render_view;
        self.sample_view = sample_view;
        self.sampler = sampler;
        self.bind_group = bind_group;
        self.width = width;
        self.height = height;
    }

    fn create_resources(
        device: &wgpu::Device,
        bind_group_layout: &wgpu::BindGroupLayout,
        width: u32,
        height: u32,
        format: wgpu::TextureFormat,
    ) -> (
        wgpu::Texture,
        wgpu::TextureView,
        wgpu::TextureView,
        wgpu::Sampler,
        wgpu::BindGroup,
    ) {
        let size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("render_target_texture"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });

        let render_view = texture.create_view(&wgpu::TextureViewDescriptor {
            label: Some("render_target_render_view"),
            ..Default::default()
        });

        let sample_view = texture.create_view(&wgpu::TextureViewDescriptor {
            label: Some("render_target_sample_view"),
            ..Default::default()
        });

        let sampler = create_nearest_sampler(device, "render_target_sampler");

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("render_target_bind_group"),
            layout: bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&sample_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });

        (texture, render_view, sample_view, sampler, bind_group)
    }
}

/// Creates a standard texture bind group layout for render target sampling.
pub fn create_texture_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("render_target_texture_bind_group_layout"),
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
    })
}
