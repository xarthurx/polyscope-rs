//! ImGui context and integration.

// TODO: Implement dear-imgui-rs integration
// This will include:
// - Creating and managing the ImGui context
// - Setting up the wgpu renderer
// - Handling input events via winit
// - Theme and style configuration

/// The UI context wrapping dear-imgui.
pub struct UiContext {
    // TODO: Add imgui context, renderer, platform
}

impl UiContext {
    /// Creates a new UI context.
    pub fn new(
        _device: &wgpu::Device,
        _queue: &wgpu::Queue,
        _format: wgpu::TextureFormat,
    ) -> Self {
        // TODO: Initialize dear-imgui-rs
        Self {}
    }
}
