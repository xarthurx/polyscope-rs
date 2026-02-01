//! egui integration with wgpu and winit.

use egui::Context;
use egui_wgpu::Renderer as EguiRenderer;
use egui_wgpu::ScreenDescriptor;
use egui_winit::State as EguiWinitState;
use winit::event::WindowEvent;
use winit::window::Window;

/// Manages egui state and rendering.
pub struct EguiIntegration {
    pub context: Context,
    pub state: EguiWinitState,
    pub renderer: EguiRenderer,
    /// Stored raw input from the most recent `begin_frame`, used for multi-pass rerun.
    last_raw_input: egui::RawInput,
}

impl EguiIntegration {
    /// Creates a new egui integration.
    #[must_use]
    pub fn new(device: &wgpu::Device, output_format: wgpu::TextureFormat, window: &Window) -> Self {
        let context = Context::default();

        // Configure dark theme
        context.set_visuals(egui::Visuals::dark());

        let viewport_id = context.viewport_id();
        let state = EguiWinitState::new(context.clone(), viewport_id, window, None, None, None);

        let renderer =
            EguiRenderer::new(device, output_format, egui_wgpu::RendererOptions::default());

        Self {
            context,
            state,
            renderer,
            last_raw_input: egui::RawInput::default(),
        }
    }

    /// Handles a winit window event.
    /// Returns true if egui consumed the event.
    pub fn handle_event(&mut self, window: &Window, event: &WindowEvent) -> bool {
        let response = self.state.on_window_event(window, event);
        response.consumed
    }

    /// Begins a new frame by collecting input events and starting the egui pass.
    pub fn begin_frame(&mut self, window: &Window) {
        let raw_input = self.state.take_egui_input(window);
        self.last_raw_input = raw_input.clone();
        self.context.begin_pass(raw_input);
    }

    /// Begins a re-run pass for multi-pass layout.
    ///
    /// egui's `Grid` widget makes itself invisible on the first frame it appears
    /// (a sizing pass) and calls `ctx.request_discard()` expecting a second pass.
    /// This method starts that second pass using `take()` on the stored raw input,
    /// which preserves `time`/`screen_rect`/`modifiers` but clears events — matching
    /// the approach used by `Context::run()`.
    pub fn begin_rerun_pass(&mut self) {
        self.context.begin_pass(self.last_raw_input.take());
    }

    /// Ends the current egui pass and returns the output without handling platform events.
    ///
    /// Use this in a multi-pass loop. Call [`Self::handle_platform_output`] once
    /// after the final pass.
    pub fn end_pass(&mut self) -> egui::FullOutput {
        self.context.end_pass()
    }

    /// Handles platform output (clipboard, cursor, IME, etc.).
    pub fn handle_platform_output(&mut self, window: &Window, output: &egui::PlatformOutput) {
        self.state.handle_platform_output(window, output.clone());
    }

    /// Ends the frame and returns paint jobs (convenience for single-pass usage).
    pub fn end_frame(&mut self, window: &Window) -> egui::FullOutput {
        let output = self.end_pass();
        self.handle_platform_output(window, &output.platform_output);
        output
    }

    /// Renders egui to the given render pass.
    pub fn render(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        screen_descriptor: &ScreenDescriptor,
        output: egui::FullOutput,
    ) {
        let paint_jobs = self
            .context
            .tessellate(output.shapes, output.pixels_per_point);

        for (id, image_delta) in &output.textures_delta.set {
            self.renderer
                .update_texture(device, queue, *id, image_delta);
        }

        self.renderer
            .update_buffers(device, queue, encoder, &paint_jobs, screen_descriptor);

        {
            let render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("egui render pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load, // Don't clear - render on top
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                ..Default::default()
            });

            // Convert to 'static lifetime as required by egui-wgpu's render method
            let mut render_pass = render_pass.forget_lifetime();

            self.renderer
                .render(&mut render_pass, &paint_jobs, screen_descriptor);
        }

        for id in &output.textures_delta.free {
            self.renderer.free_texture(id);
        }
    }
}
