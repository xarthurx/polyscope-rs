use super::RenderEngine;

impl RenderEngine {
    /// Creates the SSAO output texture (blurred result).
    pub(crate) fn create_ssao_output_texture(&mut self) {
        let ssao_output_texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("SSAO Output Texture"),
            size: wgpu::Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        let ssao_output_view =
            ssao_output_texture.create_view(&wgpu::TextureViewDescriptor::default());

        self.ssao_output_texture = Some(ssao_output_texture);
        self.ssao_output_view = Some(ssao_output_view);
    }

    /// Creates HDR texture at specified size.
    pub(crate) fn create_hdr_texture_with_size(&mut self, width: u32, height: u32) {
        let hdr_texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("HDR Render Texture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba16Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        let hdr_view = hdr_texture.create_view(&wgpu::TextureViewDescriptor::default());
        self.hdr_texture = Some(hdr_texture);
        self.hdr_view = Some(hdr_view);
    }

    /// Creates normal G-buffer texture at specified size.
    pub fn create_normal_texture_with_size(&mut self, width: u32, height: u32) {
        let normal_texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Normal G-Buffer Texture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba16Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        let normal_view = normal_texture.create_view(&wgpu::TextureViewDescriptor::default());
        self.normal_texture = Some(normal_texture);
        self.normal_view = Some(normal_view);
    }

    /// Creates SSAO output texture at specified size.
    pub(crate) fn create_ssao_output_texture_with_size(&mut self, width: u32, height: u32) {
        let ssao_output_texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("SSAO Output Texture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        let ssao_output_view =
            ssao_output_texture.create_view(&wgpu::TextureViewDescriptor::default());

        self.ssao_output_texture = Some(ssao_output_texture);
        self.ssao_output_view = Some(ssao_output_view);
    }

    /// Creates the HDR intermediate texture for tone mapping.
    pub(crate) fn create_hdr_texture(&mut self) {
        let hdr_texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("HDR Texture"),
            size: wgpu::Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba16Float, // HDR format
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        let hdr_view = hdr_texture.create_view(&wgpu::TextureViewDescriptor::default());

        self.hdr_texture = Some(hdr_texture);
        self.hdr_view = Some(hdr_view);
    }

    /// Creates the normal G-buffer texture for SSAO.
    pub(crate) fn create_normal_texture(&mut self) {
        let normal_texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Normal G-Buffer"),
            size: wgpu::Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba16Float, // View-space normals (xyz) + unused (w)
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        let normal_view = normal_texture.create_view(&wgpu::TextureViewDescriptor::default());

        self.normal_texture = Some(normal_texture);
        self.normal_view = Some(normal_view);
    }

    /// Creates the SSAO noise texture.
    pub(crate) fn create_ssao_noise_texture(&mut self) {
        use rand::Rng;

        // Generate 4x4 random rotation vectors
        let mut rng = rand::thread_rng();
        let mut noise_data = Vec::with_capacity(4 * 4 * 4); // 4x4 pixels, RGBA8

        for _ in 0..16 {
            // Random rotation vector in tangent plane (z=0)
            let angle: f32 = rng.gen_range(0.0..std::f32::consts::TAU);
            let x = angle.cos();
            let y = angle.sin();
            // Store in [0,1] range
            noise_data.push(((x * 0.5 + 0.5) * 255.0) as u8);
            noise_data.push(((y * 0.5 + 0.5) * 255.0) as u8);
            noise_data.push(0u8); // z = 0
            noise_data.push(255u8); // w = 1
        }

        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("SSAO Noise Texture"),
            size: wgpu::Extent3d {
                width: 4,
                height: 4,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        self.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &noise_data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * 4),
                rows_per_image: Some(4),
            },
            wgpu::Extent3d {
                width: 4,
                height: 4,
                depth_or_array_layers: 1,
            },
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        self.ssao_noise_texture = Some(texture);
        self.ssao_noise_view = Some(view);
    }
}
