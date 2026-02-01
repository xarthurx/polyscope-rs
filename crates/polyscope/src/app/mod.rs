//! Application window and event loop management.

mod input;
mod picking;
mod render;
mod render_init;
mod render_scene;

use std::collections::HashSet;
pub(super) use std::sync::Arc;

pub(super) use egui_wgpu::ScreenDescriptor;
pub(super) use pollster::FutureExt;
pub(super) use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::{ElementState, MouseButton, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::KeyCode,
    window::{Window, WindowId},
};

pub(super) use polyscope_core::{GroundPlaneConfig, GroundPlaneMode, Structure};
pub(super) use polyscope_render::{reflection, PickResult, RenderEngine};
pub(super) use polyscope_structures::{
    CameraView, CurveNetwork, PointCloud, SurfaceMesh, VolumeGrid, VolumeMesh,
};
pub(super) use polyscope_ui::EguiIntegration;

pub(super) use crate::Vec3;

/// The polyscope application state.
pub struct App {
    pub(super) window: Option<Arc<Window>>,
    pub(super) engine: Option<RenderEngine>,
    pub(super) egui: Option<EguiIntegration>,
    pub(super) close_requested: bool,
    pub(super) background_color: Vec3,
    // Mouse state for camera control
    // These track the PHYSICAL button state, updated on every press/release
    pub(super) mouse_pos: (f64, f64),
    pub(super) left_mouse_down: bool,
    pub(super) right_mouse_down: bool,
    // Modifier keys
    pub(super) shift_down: bool,
    // Drag tracking - accumulated distance since mouse press
    pub(super) drag_distance: f64,
    // Selection state
    pub(super) selection: Option<PickResult>,
    pub(super) last_click_pos: Option<(f64, f64)>,
    // Double-click detection
    pub(super) last_left_click_time: Option<std::time::Instant>,
    pub(super) last_left_click_screen_pos: Option<(f64, f64)>,
    // GPU picking - selected element index (from GPU pick)
    pub(super) selected_element_index: Option<u32>,
    // Ground plane settings
    pub(super) ground_plane: GroundPlaneConfig,
    // Screenshot state
    pub(super) screenshot_pending: Option<String>,
    pub(super) screenshot_counter: u32,
    // Camera settings UI state
    pub(super) camera_settings: polyscope_ui::CameraSettings,
    // Scene extents UI state
    pub(super) scene_extents: polyscope_ui::SceneExtents,
    // Appearance settings UI state
    pub(super) appearance_settings: polyscope_ui::AppearanceSettings,
    // Slice plane UI state
    pub(super) slice_plane_settings: Vec<polyscope_ui::SlicePlaneSettings>,
    pub(super) new_slice_plane_name: String,
    // Group UI state
    pub(super) group_settings: Vec<polyscope_ui::GroupSettings>,
    // Dynamic left panel width (updated each frame from egui)
    pub(super) left_panel_width: f64,
    // Gizmo UI state
    pub(super) gizmo_settings: polyscope_ui::GizmoSettings,
    pub(super) selection_info: polyscope_ui::SelectionInfo,
    // Slice plane gizmo state
    pub(super) slice_plane_selection: polyscope_ui::SlicePlaneSelectionInfo,
    // Visual gizmo
    pub(super) transform_gizmo: polyscope_ui::TransformGizmo,
    // Tone mapping settings
    pub(super) tone_mapping_settings: polyscope_ui::ToneMappingSettings,
    // Material loading UI state
    pub(super) material_load_state: polyscope_ui::MaterialLoadState,
    // Whether the camera has been auto-fitted to the scene
    pub(super) camera_fitted: bool,
    // Keyboard state for first-person WASD movement
    pub(super) keys_down: HashSet<KeyCode>,
    // Frame timing for first-person movement
    pub(super) last_frame_time: Option<std::time::Instant>,
}

impl App {
    /// Creates a new application.
    pub fn new() -> Self {
        Self {
            window: None,
            engine: None,
            egui: None,
            close_requested: false,
            background_color: Vec3::new(0.1, 0.1, 0.1),
            mouse_pos: (0.0, 0.0),
            left_mouse_down: false,
            right_mouse_down: false,
            shift_down: false,
            drag_distance: 0.0,
            selection: None,
            last_click_pos: None,
            last_left_click_time: None,
            last_left_click_screen_pos: None,
            selected_element_index: None,
            ground_plane: GroundPlaneConfig::default(),
            screenshot_pending: None,
            screenshot_counter: 0,
            camera_settings: polyscope_ui::CameraSettings::default(),
            scene_extents: polyscope_ui::SceneExtents::default(),
            appearance_settings: polyscope_ui::AppearanceSettings::default(),
            slice_plane_settings: crate::get_slice_plane_settings(),
            new_slice_plane_name: String::new(),
            group_settings: crate::get_group_settings(),
            left_panel_width: 320.0, // Default, updated dynamically each frame
            gizmo_settings: crate::get_gizmo_settings(),
            selection_info: polyscope_ui::SelectionInfo::default(),
            slice_plane_selection: polyscope_ui::SlicePlaneSelectionInfo::default(),
            transform_gizmo: polyscope_ui::TransformGizmo::new(),
            tone_mapping_settings: polyscope_ui::ToneMappingSettings::default(),
            material_load_state: polyscope_ui::MaterialLoadState::default(),
            camera_fitted: false,
            keys_down: HashSet::new(),
            last_frame_time: None,
        }
    }

    /// Requests a screenshot with an auto-generated filename.
    pub fn request_auto_screenshot(&mut self) {
        let filename = format!("screenshot_{:04}.png", self.screenshot_counter);
        self.screenshot_counter += 1;
        self.screenshot_pending = Some(filename);
    }

    /// Sets the background color.
    #[allow(dead_code)]
    pub fn set_background_color(&mut self, color: Vec3) {
        self.background_color = color;
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

/// Runs the polyscope application.
pub fn run_app() {
    let event_loop = EventLoop::new().expect("failed to create event loop");
    let mut app = App::new();

    event_loop.run_app(&mut app).expect("event loop error");
}
