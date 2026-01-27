//! Shader management.

use crate::error::{RenderError, RenderResult};

/// A compiled shader program.
pub struct ShaderProgram {
    /// The render pipeline.
    pub pipeline: wgpu::RenderPipeline,
    /// Bind group layouts.
    pub bind_group_layouts: Vec<wgpu::BindGroupLayout>,
}

/// Builder for creating shader programs.
pub struct ShaderBuilder {
    vertex_source: Option<String>,
    fragment_source: Option<String>,
    vertex_entry: String,
    fragment_entry: String,
    label: Option<String>,
}

impl ShaderBuilder {
    /// Creates a new shader builder.
    #[must_use]
    pub fn new() -> Self {
        Self {
            vertex_source: None,
            fragment_source: None,
            vertex_entry: "vs_main".to_string(),
            fragment_entry: "fs_main".to_string(),
            label: None,
        }
    }

    /// Sets the vertex shader source (WGSL).
    pub fn with_vertex(mut self, source: impl Into<String>) -> Self {
        self.vertex_source = Some(source.into());
        self
    }

    /// Sets the fragment shader source (WGSL).
    pub fn with_fragment(mut self, source: impl Into<String>) -> Self {
        self.fragment_source = Some(source.into());
        self
    }

    /// Sets the vertex shader entry point.
    pub fn with_vertex_entry(mut self, entry: impl Into<String>) -> Self {
        self.vertex_entry = entry.into();
        self
    }

    /// Sets the fragment shader entry point.
    pub fn with_fragment_entry(mut self, entry: impl Into<String>) -> Self {
        self.fragment_entry = entry.into();
        self
    }

    /// Sets the shader label for debugging.
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Builds the shader module (does not create pipeline).
    pub fn build_module(self, device: &wgpu::Device) -> RenderResult<wgpu::ShaderModule> {
        let source = self.combined_source()?;

        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: self.label.as_deref(),
            source: wgpu::ShaderSource::Wgsl(source.into()),
        });

        Ok(module)
    }

    fn combined_source(&self) -> RenderResult<String> {
        let vertex = self
            .vertex_source
            .as_ref()
            .ok_or_else(|| RenderError::ShaderCompilationFailed("missing vertex shader".into()))?;

        let fragment = self.fragment_source.as_ref().ok_or_else(|| {
            RenderError::ShaderCompilationFailed("missing fragment shader".into())
        })?;

        // If sources are the same file, just return one
        if vertex == fragment {
            return Ok(vertex.clone());
        }

        // Otherwise combine them
        Ok(format!("{vertex}\n\n{fragment}"))
    }
}

impl Default for ShaderBuilder {
    fn default() -> Self {
        Self::new()
    }
}
