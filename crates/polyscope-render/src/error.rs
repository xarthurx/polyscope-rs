//! Rendering error types.

use thiserror::Error;

/// Errors that can occur during rendering operations.
#[derive(Error, Debug)]
pub enum RenderError {
    /// Failed to create wgpu adapter.
    #[error("failed to create graphics adapter")]
    AdapterCreationFailed,

    /// Failed to create wgpu device.
    #[error("failed to create graphics device: {0}")]
    DeviceCreationFailed(#[from] wgpu::RequestDeviceError),

    /// Failed to create surface.
    #[error("failed to create surface: {0}")]
    SurfaceCreationFailed(#[from] wgpu::CreateSurfaceError),

    /// Surface configuration failed.
    #[error("surface configuration failed")]
    SurfaceConfigurationFailed,

    /// Shader compilation failed.
    #[error("shader compilation failed: {0}")]
    ShaderCompilationFailed(String),

    /// Pipeline creation failed.
    #[error("pipeline creation failed: {0}")]
    PipelineCreationFailed(String),

    /// Buffer creation failed.
    #[error("buffer creation failed: {0}")]
    BufferCreationFailed(String),

    /// Texture creation failed.
    #[error("texture creation failed: {0}")]
    TextureCreationFailed(String),

    /// Surface lost.
    #[error("surface lost")]
    SurfaceLost,

    /// Surface outdated.
    #[error("surface outdated")]
    SurfaceOutdated,

    /// Out of memory.
    #[error("out of memory")]
    OutOfMemory,

    /// Timeout waiting for GPU.
    #[error("timeout waiting for GPU")]
    Timeout,
}

/// A specialized Result type for rendering operations.
pub type RenderResult<T> = std::result::Result<T, RenderError>;
