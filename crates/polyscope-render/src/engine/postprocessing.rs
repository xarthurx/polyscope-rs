use super::RenderEngine;
use crate::tone_mapping::ToneMapPass;

impl RenderEngine {
    /// Creates a screenshot texture for capturing frames.
    ///
    /// Returns a texture view (HDR format) that can be used as a render target.
    /// The pipelines render to HDR format, so we need an HDR texture for rendering,
    /// then tone map to the final screenshot texture.
    /// After rendering to this view, call `apply_screenshot_tone_mapping()` then
    /// `capture_screenshot()` to get the pixel data.
    pub fn create_screenshot_target(&mut self) -> wgpu::TextureView {
        // Calculate buffer size with proper alignment
        let bytes_per_row = Self::aligned_bytes_per_row(self.width);
        let buffer_size = u64::from(bytes_per_row * self.height);

        // Create HDR texture for rendering (matches pipeline format)
        let hdr_texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("screenshot HDR texture"),
            size: wgpu::Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba16Float, // HDR format matching pipelines
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        let hdr_view = hdr_texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Create final capture texture (surface format for readback)
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("screenshot texture"),
            size: wgpu::Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: self.surface_config.format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });

        // Create staging buffer for readback
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("screenshot buffer"),
            size: buffer_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        self.screenshot_hdr_texture = Some(hdr_texture);
        self.screenshot_hdr_view = Some(hdr_view);
        self.screenshot_texture = Some(texture);
        self.screenshot_buffer = Some(buffer);

        // Return the HDR view for rendering
        self.screenshot_hdr_view.as_ref().unwrap().clone()
    }

    /// Returns the screenshot texture view (for tone mapping output).
    pub fn screenshot_texture_view(&self) -> Option<wgpu::TextureView> {
        self.screenshot_texture
            .as_ref()
            .map(|t| t.create_view(&wgpu::TextureViewDescriptor::default()))
    }

    /// Applies tone mapping from the screenshot HDR texture to the final screenshot texture.
    pub fn apply_screenshot_tone_mapping(&mut self, encoder: &mut wgpu::CommandEncoder) {
        let Some(hdr_view) = &self.screenshot_hdr_view else {
            log::error!("Screenshot HDR view not initialized");
            return;
        };

        let Some(screenshot_texture) = &self.screenshot_texture else {
            log::error!("Screenshot texture not initialized");
            return;
        };

        let screenshot_view =
            screenshot_texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Use the existing tone mapping pass
        // For screenshots, we use the main SSAO output view if available
        // (Note: SSAO effect depends on the main render resolution, not screenshot resolution)
        if let Some(tone_map_pass) = &self.tone_map_pass {
            // Use SSAO output or fall back to HDR view (which is ignored when ssao_enabled=false)
            let ssao_view = self.ssao_output_view.as_ref().unwrap_or(hdr_view);
            tone_map_pass.render_to_target(
                &self.device,
                encoder,
                hdr_view,
                ssao_view,
                &screenshot_view,
            );
        }
    }

    /// Returns the screenshot depth view for rendering.
    pub fn screenshot_depth_view(&self) -> &wgpu::TextureView {
        &self.depth_view
    }

    /// Calculates bytes per row with proper alignment for wgpu buffer copies.
    fn aligned_bytes_per_row(width: u32) -> u32 {
        let bytes_per_pixel = 4u32; // RGBA8
        let unaligned = width * bytes_per_pixel;
        let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        unaligned.div_ceil(align) * align
    }

    /// Captures the screenshot after rendering to the screenshot target.
    ///
    /// This method copies the screenshot texture to a buffer and reads it back.
    /// Call this after rendering to the view returned by `create_screenshot_target()`.
    ///
    /// Returns the raw RGBA pixel data.
    pub fn capture_screenshot(&mut self) -> Result<Vec<u8>, crate::screenshot::ScreenshotError> {
        let texture = self
            .screenshot_texture
            .as_ref()
            .ok_or(crate::screenshot::ScreenshotError::InvalidImageData)?;
        let buffer = self
            .screenshot_buffer
            .as_ref()
            .ok_or(crate::screenshot::ScreenshotError::InvalidImageData)?;

        let bytes_per_row = Self::aligned_bytes_per_row(self.width);

        // Create encoder and copy texture to buffer
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("screenshot copy encoder"),
            });

        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(bytes_per_row),
                    rows_per_image: Some(self.height),
                },
            },
            wgpu::Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
        );

        self.queue.submit(std::iter::once(encoder.finish()));

        // Map buffer and read data
        let buffer_slice = buffer.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            tx.send(result).unwrap();
        });
        let _ = self.device.poll(wgpu::PollType::wait_indefinitely());
        rx.recv()
            .map_err(|_| crate::screenshot::ScreenshotError::BufferMapFailed)?
            .map_err(|_| crate::screenshot::ScreenshotError::BufferMapFailed)?;

        // Copy data, removing row padding
        let data = buffer_slice.get_mapped_range();
        let mut result = Vec::with_capacity((self.width * self.height * 4) as usize);
        let row_bytes = (self.width * 4) as usize;

        for row in 0..self.height {
            let start = (row * bytes_per_row) as usize;
            let end = start + row_bytes;
            result.extend_from_slice(&data[start..end]);
        }

        drop(data);
        buffer.unmap();

        // Clean up screenshot resources
        self.screenshot_texture = None;
        self.screenshot_buffer = None;
        self.screenshot_hdr_texture = None;
        self.screenshot_hdr_view = None;

        Ok(result)
    }

    /// Initializes tone mapping resources.
    pub(crate) fn init_tone_mapping(&mut self) {
        self.tone_map_pass = Some(ToneMapPass::new(&self.device, self.surface_config.format));
        self.create_hdr_texture();
        self.create_normal_texture();
        self.create_ssao_noise_texture();
        self.init_ssao_pass();
    }

    /// Initializes SSAO pass.
    pub(crate) fn init_ssao_pass(&mut self) {
        let ssao_pass = crate::ssao_pass::SsaoPass::new(&self.device, self.width, self.height);
        self.ssao_pass = Some(ssao_pass);
        self.create_ssao_output_texture();
    }

    /// Initializes SSAA (supersampling) pass.
    /// The pipeline uses `Rgba16Float` because it downsamples the HDR texture
    /// to the HDR intermediate texture (both are `Rgba16Float`).
    pub(crate) fn init_ssaa_pass(&mut self) {
        self.ssaa_pass = Some(crate::ssaa_pass::SsaaPass::new(
            &self.device,
            wgpu::TextureFormat::Rgba16Float,
        ));
    }

    /// Returns the current SSAA factor (1 = off, 2 = 2x, 4 = 4x).
    #[must_use]
    pub fn ssaa_factor(&self) -> u32 {
        self.ssaa_factor
    }

    /// Sets the SSAA factor and recreates render textures at the new resolution.
    /// Valid values are 1 (off), 2 (2x supersampling), or 4 (4x supersampling).
    pub fn set_ssaa_factor(&mut self, factor: u32) {
        let factor = factor.clamp(1, 4);
        if factor == self.ssaa_factor {
            return;
        }

        // Wait for any in-flight GPU work before destroying textures
        let _ = self.device.poll(wgpu::PollType::wait_indefinitely());

        self.ssaa_factor = factor;

        // Update SSAA pass uniform
        if let Some(ref mut ssaa_pass) = self.ssaa_pass {
            ssaa_pass.set_ssaa_factor(&self.queue, factor);
        }

        // Recreate all resolution-dependent textures at SSAA resolution
        self.recreate_ssaa_textures();
    }

    /// Recreates all resolution-dependent textures at SSAA resolution.
    pub(crate) fn recreate_ssaa_textures(&mut self) {
        let ssaa_width = self.width * self.ssaa_factor;
        let ssaa_height = self.height * self.ssaa_factor;

        // Recreate depth texture at SSAA resolution
        let (depth_texture, depth_view, depth_only_view) =
            Self::create_depth_texture(&self.device, ssaa_width, ssaa_height);
        self.depth_texture = depth_texture;
        self.depth_view = depth_view;
        self.depth_only_view = depth_only_view;

        // Recreate HDR texture at SSAA resolution
        self.create_hdr_texture_with_size(ssaa_width, ssaa_height);

        // Recreate normal G-buffer at SSAA resolution
        self.create_normal_texture_with_size(ssaa_width, ssaa_height);

        // Recreate SSAO output at SSAA resolution
        self.create_ssao_output_texture_with_size(ssaa_width, ssaa_height);

        // Resize SSAO pass
        if let Some(ref mut ssao_pass) = self.ssao_pass {
            ssao_pass.resize(&self.device, &self.queue, ssaa_width, ssaa_height);
        }

        // Create intermediate texture for downsampling (at screen resolution)
        if self.ssaa_factor > 1 {
            self.create_ssaa_intermediate_texture();
        } else {
            self.ssaa_intermediate_texture = None;
            self.ssaa_intermediate_view = None;
        }
    }

    /// Creates the intermediate texture for SSAA downsampling (at screen resolution).
    pub(crate) fn create_ssaa_intermediate_texture(&mut self) {
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("SSAA Intermediate Texture"),
            size: wgpu::Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba16Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        self.ssaa_intermediate_texture = Some(texture);
        self.ssaa_intermediate_view = Some(view);
    }

    /// Ensures the depth peel pass is initialized and matches render resolution.
    pub fn ensure_depth_peel_pass(&mut self) {
        let (render_w, render_h) = self.render_dimensions();

        if self.mesh_bind_group_layout.is_none() {
            self.create_mesh_pipeline();
        }

        if let Some(ref mut pass) = self.depth_peel_pass {
            pass.resize(&self.device, render_w, render_h);
        } else {
            self.depth_peel_pass = Some(crate::depth_peel_pass::DepthPeelPass::new(
                &self.device,
                render_w,
                render_h,
                self.mesh_bind_group_layout.as_ref().unwrap(),
                &self.slice_plane_bind_group_layout,
                &self.matcap_bind_group_layout,
            ));
        }
    }

    /// Returns the depth peel pass, if initialized.
    pub fn depth_peel_pass(&self) -> Option<&crate::depth_peel_pass::DepthPeelPass> {
        self.depth_peel_pass.as_ref()
    }

    /// Returns a mutable reference to the depth peel pass, if initialized.
    pub fn depth_peel_pass_mut(&mut self) -> Option<&mut crate::depth_peel_pass::DepthPeelPass> {
        self.depth_peel_pass.as_mut()
    }

    /// Returns the HDR texture view for rendering the scene.
    pub fn hdr_view(&self) -> Option<&wgpu::TextureView> {
        self.hdr_view.as_ref()
    }

    /// Returns the normal G-buffer view if available.
    pub fn normal_view(&self) -> Option<&wgpu::TextureView> {
        self.normal_view.as_ref()
    }

    /// Returns the SSAO noise texture view if available.
    pub fn ssao_noise_view(&self) -> Option<&wgpu::TextureView> {
        self.ssao_noise_view.as_ref()
    }

    /// Returns the SSAO output texture view if available.
    pub fn ssao_output_view(&self) -> Option<&wgpu::TextureView> {
        self.ssao_output_view.as_ref()
    }

    /// Returns the SSAO pass.
    pub fn ssao_pass(&self) -> Option<&crate::ssao_pass::SsaoPass> {
        self.ssao_pass.as_ref()
    }

    /// Renders the SSAO pass.
    /// Returns true if SSAO was rendered, false if resources are not available.
    pub fn render_ssao(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        config: &polyscope_core::SsaoConfig,
    ) -> bool {
        // Check if all required resources are available
        // Use depth_only_view for SSAO (excludes stencil aspect)
        let (ssao_pass, depth_view, normal_view, noise_view, output_view) = match (
            &self.ssao_pass,
            Some(&self.depth_only_view),
            self.normal_view.as_ref(),
            self.ssao_noise_view.as_ref(),
            self.ssao_output_view.as_ref(),
        ) {
            (Some(pass), Some(depth), Some(normal), Some(noise), Some(output)) => {
                (pass, depth, normal, noise, output)
            }
            _ => return false,
        };

        if !config.enabled {
            return false;
        }

        // Update SSAO uniforms — use SSAA-scaled dimensions since
        // SSAO textures are rendered at SSAA resolution
        let (render_w, render_h) = self.render_dimensions();
        let proj = self.camera.projection_matrix();
        let inv_proj = proj.inverse();
        ssao_pass.update_uniforms(
            &self.queue,
            proj,
            inv_proj,
            config.radius,
            config.bias,
            config.intensity,
            config.sample_count,
            render_w as f32,
            render_h as f32,
        );

        // Create bind groups
        let ssao_bind_group =
            ssao_pass.create_ssao_bind_group(&self.device, depth_view, normal_view, noise_view);
        // Blur bind group now includes depth view for edge-aware bilateral filtering
        let blur_bind_group = ssao_pass.create_blur_bind_group(&self.device, depth_view);

        // Render SSAO pass
        ssao_pass.render_ssao(encoder, &ssao_bind_group);

        // Render blur pass to output texture
        ssao_pass.render_blur(encoder, output_view, &blur_bind_group);

        true
    }

    /// Returns the tone map pass.
    pub fn tone_map_pass(&self) -> Option<&ToneMapPass> {
        self.tone_map_pass.as_ref()
    }

    /// Updates tone mapping uniforms.
    pub fn update_tone_mapping(
        &self,
        exposure: f32,
        white_level: f32,
        gamma: f32,
        ssao_enabled: bool,
    ) {
        if let Some(tone_map) = &self.tone_map_pass {
            tone_map.update_uniforms(&self.queue, exposure, white_level, gamma, ssao_enabled);
        }
    }

    /// Renders the tone mapping pass from HDR to the output view.
    /// Uses SSAO texture if available, otherwise uses a default white texture.
    ///
    /// When SSAA is enabled (factor > 1):
    /// 1. Downsamples HDR (SSAA res) → intermediate HDR (screen res)
    /// 2. Tone maps intermediate HDR → output LDR (SSAO disabled — resolution mismatch)
    pub fn render_tone_mapping(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        output_view: &wgpu::TextureView,
    ) {
        if let (Some(tone_map), Some(hdr_view)) = (&self.tone_map_pass, &self.hdr_view) {
            // If SSAA is enabled, first downsample HDR, then tone map
            if self.ssaa_factor > 1 {
                if let (Some(intermediate_view), Some(ssaa_pass)) =
                    (&self.ssaa_intermediate_view, &self.ssaa_pass)
                {
                    // Step 1: Downsample HDR (SSAA res) -> intermediate HDR (screen res)
                    ssaa_pass.render_to_target(&self.device, encoder, hdr_view, intermediate_view);

                    // Step 2: Tone map intermediate HDR -> output LDR
                    // Pass intermediate_view as the SSAO slot — SSAO is disabled via
                    // ssao_enabled=0 uniform so the texture value is ignored, but the
                    // bind group requires a valid Float texture of matching format.
                    let bind_group = tone_map.create_bind_group(
                        &self.device,
                        intermediate_view,
                        intermediate_view,
                    );
                    tone_map.render(encoder, output_view, &bind_group);
                    return;
                }
            }

            // No SSAA - tone map directly from HDR to output with SSAO
            let ssao_view = self.ssao_output_view.as_ref().unwrap_or(hdr_view);
            let bind_group = tone_map.create_bind_group(&self.device, hdr_view, ssao_view);
            tone_map.render(encoder, output_view, &bind_group);
        }
    }
}
