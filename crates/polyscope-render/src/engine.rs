//! The main rendering engine.

use std::sync::Arc;

use crate::error::{RenderError, RenderResult};
use crate::camera::Camera;
use crate::color_maps::ColorMapRegistry;
use crate::materials::MaterialRegistry;

/// The main rendering engine backed by wgpu.
pub struct RenderEngine {
    /// The wgpu instance.
    pub instance: wgpu::Instance,
    /// The wgpu adapter.
    pub adapter: wgpu::Adapter,
    /// The wgpu device.
    pub device: wgpu::Device,
    /// The wgpu queue.
    pub queue: wgpu::Queue,
    /// The render surface (None for headless).
    pub surface: Option<wgpu::Surface<'static>>,
    /// Surface configuration.
    pub surface_config: wgpu::SurfaceConfiguration,
    /// Depth texture.
    pub depth_texture: wgpu::Texture,
    /// Depth texture view.
    pub depth_view: wgpu::TextureView,
    /// Material registry.
    pub materials: MaterialRegistry,
    /// Color map registry.
    pub color_maps: ColorMapRegistry,
    /// Main camera.
    pub camera: Camera,
    /// Current viewport width.
    pub width: u32,
    /// Current viewport height.
    pub height: u32,
}

impl RenderEngine {
    /// Creates a new windowed render engine.
    pub async fn new_windowed(
        window: Arc<winit::window::Window>,
    ) -> RenderResult<Self> {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        let surface = instance.create_surface(window.clone())?;

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .ok_or(RenderError::AdapterCreationFailed)?;

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("polyscope device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    memory_hints: Default::default(),
                },
                None,
            )
            .await?;

        let size = window.inner_size();
        let width = size.width.max(1);
        let height = size.height.max(1);

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);

        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width,
            height,
            present_mode: wgpu::PresentMode::AutoVsync,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &surface_config);

        let (depth_texture, depth_view) = Self::create_depth_texture(&device, width, height);

        let camera = Camera::new(width as f32 / height as f32);

        Ok(Self {
            instance,
            adapter,
            device,
            queue,
            surface: Some(surface),
            surface_config,
            depth_texture,
            depth_view,
            materials: MaterialRegistry::new(),
            color_maps: ColorMapRegistry::new(),
            camera,
            width,
            height,
        })
    }

    /// Creates a new headless render engine.
    pub async fn new_headless(width: u32, height: u32) -> RenderResult<Self> {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: None,
                force_fallback_adapter: false,
            })
            .await
            .ok_or(RenderError::AdapterCreationFailed)?;

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("polyscope device (headless)"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    memory_hints: Default::default(),
                },
                None,
            )
            .await?;

        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            width,
            height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        let (depth_texture, depth_view) = Self::create_depth_texture(&device, width, height);

        let camera = Camera::new(width as f32 / height as f32);

        Ok(Self {
            instance,
            adapter,
            device,
            queue,
            surface: None,
            surface_config,
            depth_texture,
            depth_view,
            materials: MaterialRegistry::new(),
            color_maps: ColorMapRegistry::new(),
            camera,
            width,
            height,
        })
    }

    /// Resizes the render target.
    pub fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }

        self.width = width;
        self.height = height;
        self.surface_config.width = width;
        self.surface_config.height = height;

        if let Some(ref surface) = self.surface {
            surface.configure(&self.device, &self.surface_config);
        }

        let (depth_texture, depth_view) = Self::create_depth_texture(&self.device, width, height);
        self.depth_texture = depth_texture;
        self.depth_view = depth_view;

        self.camera.set_aspect_ratio(width as f32 / height as f32);
    }

    fn create_depth_texture(
        device: &wgpu::Device,
        width: u32,
        height: u32,
    ) -> (wgpu::Texture, wgpu::TextureView) {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("depth texture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        (texture, view)
    }
}
