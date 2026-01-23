# UI Enhancement Phase A: Core View Controls Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add comprehensive camera and view controls to polyscope-rs UI matching the original C++ Polyscope functionality.

**Architecture:** Add camera configuration types (navigation style, projection mode, directions) to the render crate, then create UI panels in polyscope-ui that expose these settings. The UI panels connect to both Camera (render) and Context (core) for scene extents.

**Tech Stack:** Rust, wgpu, egui, glam

---

## Task 1: Add Camera Configuration Types to Render Crate

**Files:**
- Modify: `crates/polyscope-render/src/camera.rs:1-119`
- Modify: `crates/polyscope-render/src/lib.rs:24` (add exports)

**Step 1: Add navigation style, projection mode, and direction enums**

Add these types at the top of `crates/polyscope-render/src/camera.rs` (after line 3):

```rust
/// Camera navigation/interaction style.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NavigationStyle {
    /// Turntable - orbits around target, constrained to up direction.
    #[default]
    Turntable,
    /// Free - unconstrained rotation.
    Free,
    /// Planar - 2D panning only.
    Planar,
    /// First person - WASD-style movement.
    FirstPerson,
    /// None - camera controls disabled.
    None,
}

/// Camera projection mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ProjectionMode {
    /// Perspective projection.
    #[default]
    Perspective,
    /// Orthographic projection.
    Orthographic,
}

/// Axis direction for up/front vectors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AxisDirection {
    /// Positive X axis.
    PosX,
    /// Negative X axis.
    NegX,
    /// Positive Y axis (default up).
    #[default]
    PosY,
    /// Negative Y axis.
    NegY,
    /// Positive Z axis.
    PosZ,
    /// Negative Z axis (default front).
    NegZ,
}

impl AxisDirection {
    /// Returns the unit vector for this direction.
    pub fn to_vec3(self) -> Vec3 {
        match self {
            AxisDirection::PosX => Vec3::X,
            AxisDirection::NegX => Vec3::NEG_X,
            AxisDirection::PosY => Vec3::Y,
            AxisDirection::NegY => Vec3::NEG_Y,
            AxisDirection::PosZ => Vec3::Z,
            AxisDirection::NegZ => Vec3::NEG_Z,
        }
    }

    /// Returns display name.
    pub fn name(self) -> &'static str {
        match self {
            AxisDirection::PosX => "+X",
            AxisDirection::NegX => "-X",
            AxisDirection::PosY => "+Y",
            AxisDirection::NegY => "-Y",
            AxisDirection::PosZ => "+Z",
            AxisDirection::NegZ => "-Z",
        }
    }
}
```

**Step 2: Update Camera struct with new fields**

Modify the Camera struct in `crates/polyscope-render/src/camera.rs` to add new fields after `far: f32`:

```rust
    /// Navigation style.
    pub navigation_style: NavigationStyle,
    /// Projection mode.
    pub projection_mode: ProjectionMode,
    /// Up direction.
    pub up_direction: AxisDirection,
    /// Front direction.
    pub front_direction: AxisDirection,
    /// Movement speed multiplier.
    pub move_speed: f32,
    /// Orthographic scale (used when projection_mode is Orthographic).
    pub ortho_scale: f32,
```

**Step 3: Update Camera::new() default initialization**

Update the `Camera::new()` function to initialize the new fields:

```rust
    pub fn new(aspect_ratio: f32) -> Self {
        Self {
            position: Vec3::new(0.0, 0.0, 3.0),
            target: Vec3::ZERO,
            up: Vec3::Y,
            fov: std::f32::consts::FRAC_PI_4, // 45 degrees
            aspect_ratio,
            near: 0.01,
            far: 1000.0,
            navigation_style: NavigationStyle::Turntable,
            projection_mode: ProjectionMode::Perspective,
            up_direction: AxisDirection::PosY,
            front_direction: AxisDirection::NegZ,
            move_speed: 1.0,
            ortho_scale: 1.0,
        }
    }
```

**Step 4: Update Default impl**

Update the `Default` impl for Camera:

```rust
impl Default for Camera {
    fn default() -> Self {
        Self::new(16.0 / 9.0)
    }
}
```

**Step 5: Update projection_matrix() to support orthographic**

Replace the `projection_matrix` method:

```rust
    /// Returns the projection matrix.
    pub fn projection_matrix(&self) -> Mat4 {
        match self.projection_mode {
            ProjectionMode::Perspective => {
                Mat4::perspective_rh(self.fov, self.aspect_ratio, self.near, self.far)
            }
            ProjectionMode::Orthographic => {
                let half_height = self.ortho_scale;
                let half_width = half_height * self.aspect_ratio;
                Mat4::orthographic_rh(-half_width, half_width, -half_height, half_height, self.near, self.far)
            }
        }
    }
```

**Step 6: Add setter methods for new fields**

Add these methods to the Camera impl block:

```rust
    /// Sets the navigation style.
    pub fn set_navigation_style(&mut self, style: NavigationStyle) {
        self.navigation_style = style;
    }

    /// Sets the projection mode.
    pub fn set_projection_mode(&mut self, mode: ProjectionMode) {
        self.projection_mode = mode;
    }

    /// Sets the up direction and updates the up vector.
    pub fn set_up_direction(&mut self, direction: AxisDirection) {
        self.up_direction = direction;
        self.up = direction.to_vec3();
    }

    /// Sets the movement speed.
    pub fn set_move_speed(&mut self, speed: f32) {
        self.move_speed = speed.max(0.01);
    }

    /// Sets the orthographic scale.
    pub fn set_ortho_scale(&mut self, scale: f32) {
        self.ortho_scale = scale.max(0.01);
    }

    /// Sets the field of view in radians.
    pub fn set_fov(&mut self, fov: f32) {
        self.fov = fov.clamp(0.1, std::f32::consts::PI - 0.1);
    }

    /// Sets the near clipping plane.
    pub fn set_near(&mut self, near: f32) {
        self.near = near.max(0.001);
    }

    /// Sets the far clipping plane.
    pub fn set_far(&mut self, far: f32) {
        self.far = far.max(self.near + 0.1);
    }

    /// Returns FOV in degrees.
    pub fn fov_degrees(&self) -> f32 {
        self.fov.to_degrees()
    }

    /// Sets FOV from degrees.
    pub fn set_fov_degrees(&mut self, degrees: f32) {
        self.set_fov(degrees.to_radians());
    }
```

**Step 7: Update lib.rs exports**

Add to `crates/polyscope-render/src/lib.rs` line 24 (after `pub use camera::Camera;`):

```rust
pub use camera::{Camera, NavigationStyle, ProjectionMode, AxisDirection};
```

**Step 8: Run tests to verify compilation**

Run: `cargo test -p polyscope-render`
Expected: All tests pass (compilation check)

**Step 9: Commit**

```bash
git add crates/polyscope-render/src/camera.rs crates/polyscope-render/src/lib.rs
git commit -m "feat(render): add camera navigation style, projection mode, and direction types"
```

---

## Task 2: Add Camera Configuration Tests

**Files:**
- Modify: `crates/polyscope-render/src/camera.rs` (add tests at end)

**Step 1: Add tests for new camera functionality**

Add to end of `crates/polyscope-render/src/camera.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_axis_direction_to_vec3() {
        assert_eq!(AxisDirection::PosX.to_vec3(), Vec3::X);
        assert_eq!(AxisDirection::NegX.to_vec3(), Vec3::NEG_X);
        assert_eq!(AxisDirection::PosY.to_vec3(), Vec3::Y);
        assert_eq!(AxisDirection::NegY.to_vec3(), Vec3::NEG_Y);
        assert_eq!(AxisDirection::PosZ.to_vec3(), Vec3::Z);
        assert_eq!(AxisDirection::NegZ.to_vec3(), Vec3::NEG_Z);
    }

    #[test]
    fn test_camera_defaults() {
        let camera = Camera::default();
        assert_eq!(camera.navigation_style, NavigationStyle::Turntable);
        assert_eq!(camera.projection_mode, ProjectionMode::Perspective);
        assert_eq!(camera.up_direction, AxisDirection::PosY);
        assert_eq!(camera.move_speed, 1.0);
    }

    #[test]
    fn test_projection_mode_perspective() {
        let camera = Camera::new(1.0);
        let proj = camera.projection_matrix();
        // Perspective matrix has non-zero w division
        assert!(proj.w_axis.z != 0.0);
    }

    #[test]
    fn test_projection_mode_orthographic() {
        let mut camera = Camera::new(1.0);
        camera.projection_mode = ProjectionMode::Orthographic;
        camera.ortho_scale = 5.0;
        let proj = camera.projection_matrix();
        // Orthographic matrix has w_axis.w = 1.0, w_axis.z = 0.0
        assert!((proj.w_axis.w - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_set_fov_clamping() {
        let mut camera = Camera::new(1.0);
        camera.set_fov(0.0); // Too small
        assert!(camera.fov >= 0.1);

        camera.set_fov(std::f32::consts::PI); // Too large
        assert!(camera.fov < std::f32::consts::PI);
    }

    #[test]
    fn test_fov_degrees_conversion() {
        let mut camera = Camera::new(1.0);
        camera.set_fov_degrees(90.0);
        assert!((camera.fov_degrees() - 90.0).abs() < 0.1);
    }

    #[test]
    fn test_set_up_direction() {
        let mut camera = Camera::new(1.0);
        camera.set_up_direction(AxisDirection::PosZ);
        assert_eq!(camera.up, Vec3::Z);
        assert_eq!(camera.up_direction, AxisDirection::PosZ);
    }
}
```

**Step 2: Run tests**

Run: `cargo test -p polyscope-render`
Expected: All tests pass

**Step 3: Commit**

```bash
git add crates/polyscope-render/src/camera.rs
git commit -m "test(render): add camera configuration tests"
```

---

## Task 3: Create Camera Settings UI Panel

**Files:**
- Modify: `crates/polyscope-ui/src/panels.rs:1-192`
- Modify: `crates/polyscope-ui/src/lib.rs`

**Step 1: Add camera panel imports**

At top of `crates/polyscope-ui/src/panels.rs`, update the use statement:

```rust
use egui::{CollapsingHeader, Context, SidePanel, Ui, DragValue, Slider};
```

**Step 2: Add CameraSettings struct for UI state**

Add after line 4 (after imports):

```rust
/// Camera settings exposed in UI.
#[derive(Debug, Clone)]
pub struct CameraSettings {
    /// Navigation style (0=Turntable, 1=Free, 2=Planar, 3=FirstPerson, 4=None)
    pub navigation_style: u32,
    /// Projection mode (0=Perspective, 1=Orthographic)
    pub projection_mode: u32,
    /// Up direction (0=+X, 1=-X, 2=+Y, 3=-Y, 4=+Z, 5=-Z)
    pub up_direction: u32,
    /// Front direction
    pub front_direction: u32,
    /// Field of view in degrees
    pub fov_degrees: f32,
    /// Near clip plane
    pub near: f32,
    /// Far clip plane
    pub far: f32,
    /// Movement speed
    pub move_speed: f32,
    /// Orthographic scale
    pub ortho_scale: f32,
}

impl Default for CameraSettings {
    fn default() -> Self {
        Self {
            navigation_style: 0, // Turntable
            projection_mode: 0,  // Perspective
            up_direction: 2,     // +Y
            front_direction: 5,  // -Z
            fov_degrees: 45.0,
            near: 0.01,
            far: 1000.0,
            move_speed: 1.0,
            ortho_scale: 1.0,
        }
    }
}
```

**Step 3: Add build_camera_settings_section function**

Add after `build_controls_section` (around line 31):

```rust
/// Builds the camera settings section.
/// Returns true if any setting changed.
pub fn build_camera_settings_section(ui: &mut Ui, settings: &mut CameraSettings) -> bool {
    let mut changed = false;

    CollapsingHeader::new("Camera")
        .default_open(false)
        .show(ui, |ui| {
            // Navigation style
            egui::ComboBox::from_label("Navigation")
                .selected_text(match settings.navigation_style {
                    0 => "Turntable",
                    1 => "Free",
                    2 => "Planar",
                    3 => "First Person",
                    _ => "None",
                })
                .show_ui(ui, |ui| {
                    for (i, name) in ["Turntable", "Free", "Planar", "First Person", "None"].iter().enumerate() {
                        if ui.selectable_value(&mut settings.navigation_style, i as u32, *name).changed() {
                            changed = true;
                        }
                    }
                });

            // Projection mode
            egui::ComboBox::from_label("Projection")
                .selected_text(if settings.projection_mode == 0 { "Perspective" } else { "Orthographic" })
                .show_ui(ui, |ui| {
                    if ui.selectable_value(&mut settings.projection_mode, 0, "Perspective").changed() {
                        changed = true;
                    }
                    if ui.selectable_value(&mut settings.projection_mode, 1, "Orthographic").changed() {
                        changed = true;
                    }
                });

            ui.separator();

            // Up direction
            let directions = ["+X", "-X", "+Y", "-Y", "+Z", "-Z"];
            egui::ComboBox::from_label("Up")
                .selected_text(directions[settings.up_direction as usize])
                .show_ui(ui, |ui| {
                    for (i, name) in directions.iter().enumerate() {
                        if ui.selectable_value(&mut settings.up_direction, i as u32, *name).changed() {
                            changed = true;
                        }
                    }
                });

            // Front direction
            egui::ComboBox::from_label("Front")
                .selected_text(directions[settings.front_direction as usize])
                .show_ui(ui, |ui| {
                    for (i, name) in directions.iter().enumerate() {
                        if ui.selectable_value(&mut settings.front_direction, i as u32, *name).changed() {
                            changed = true;
                        }
                    }
                });

            ui.separator();

            // FOV (only for perspective)
            if settings.projection_mode == 0 {
                ui.horizontal(|ui| {
                    ui.label("FOV:");
                    if ui.add(Slider::new(&mut settings.fov_degrees, 10.0..=170.0).suffix("°")).changed() {
                        changed = true;
                    }
                });
            } else {
                // Ortho scale
                ui.horizontal(|ui| {
                    ui.label("Scale:");
                    if ui.add(DragValue::new(&mut settings.ortho_scale).speed(0.1).range(0.1..=100.0)).changed() {
                        changed = true;
                    }
                });
            }

            // Clip planes
            ui.horizontal(|ui| {
                ui.label("Near:");
                if ui.add(DragValue::new(&mut settings.near).speed(0.001).range(0.001..=10.0)).changed() {
                    changed = true;
                }
            });

            ui.horizontal(|ui| {
                ui.label("Far:");
                if ui.add(DragValue::new(&mut settings.far).speed(1.0).range(10.0..=10000.0)).changed() {
                    changed = true;
                }
            });

            // Move speed
            ui.horizontal(|ui| {
                ui.label("Move Speed:");
                if ui.add(DragValue::new(&mut settings.move_speed).speed(0.1).range(0.1..=10.0)).changed() {
                    changed = true;
                }
            });
        });

    changed
}
```

**Step 4: Export CameraSettings from lib.rs**

Update `crates/polyscope-ui/src/lib.rs` to add the export:

```rust
pub use panels::CameraSettings;
```

**Step 5: Run compilation check**

Run: `cargo check -p polyscope-ui`
Expected: No errors

**Step 6: Commit**

```bash
git add crates/polyscope-ui/src/panels.rs crates/polyscope-ui/src/lib.rs
git commit -m "feat(ui): add camera settings panel"
```

---

## Task 4: Create Scene Extents UI Panel

**Files:**
- Modify: `crates/polyscope-ui/src/panels.rs`

**Step 1: Add SceneExtents struct**

Add after `CameraSettings` struct:

```rust
/// Scene extents information for UI display.
#[derive(Debug, Clone, Default)]
pub struct SceneExtents {
    /// Whether to auto-compute extents.
    pub auto_compute: bool,
    /// Length scale of the scene.
    pub length_scale: f32,
    /// Bounding box minimum.
    pub bbox_min: [f32; 3],
    /// Bounding box maximum.
    pub bbox_max: [f32; 3],
}
```

**Step 2: Add build_scene_extents_section function**

Add after `build_camera_settings_section`:

```rust
/// Builds the scene extents section.
/// Returns true if auto_compute changed.
pub fn build_scene_extents_section(ui: &mut Ui, extents: &mut SceneExtents) -> bool {
    let mut changed = false;

    CollapsingHeader::new("Scene Extents")
        .default_open(false)
        .show(ui, |ui| {
            if ui.checkbox(&mut extents.auto_compute, "Auto-compute").changed() {
                changed = true;
            }

            ui.separator();

            // Display length scale (read-only)
            ui.horizontal(|ui| {
                ui.label("Length scale:");
                ui.label(format!("{:.4}", extents.length_scale));
            });

            // Display bounding box (read-only)
            ui.label("Bounding box:");
            ui.indent("bbox", |ui| {
                ui.horizontal(|ui| {
                    ui.label("Min:");
                    ui.label(format!(
                        "({:.2}, {:.2}, {:.2})",
                        extents.bbox_min[0], extents.bbox_min[1], extents.bbox_min[2]
                    ));
                });
                ui.horizontal(|ui| {
                    ui.label("Max:");
                    ui.label(format!(
                        "({:.2}, {:.2}, {:.2})",
                        extents.bbox_max[0], extents.bbox_max[1], extents.bbox_max[2]
                    ));
                });
            });

            // Compute center and size
            let center = [
                (extents.bbox_min[0] + extents.bbox_max[0]) / 2.0,
                (extents.bbox_min[1] + extents.bbox_max[1]) / 2.0,
                (extents.bbox_min[2] + extents.bbox_max[2]) / 2.0,
            ];
            ui.horizontal(|ui| {
                ui.label("Center:");
                ui.label(format!("({:.2}, {:.2}, {:.2})", center[0], center[1], center[2]));
            });
        });

    changed
}
```

**Step 3: Export SceneExtents from lib.rs**

Update `crates/polyscope-ui/src/lib.rs`:

```rust
pub use panels::{CameraSettings, SceneExtents};
```

**Step 4: Run compilation check**

Run: `cargo check -p polyscope-ui`
Expected: No errors

**Step 5: Commit**

```bash
git add crates/polyscope-ui/src/panels.rs crates/polyscope-ui/src/lib.rs
git commit -m "feat(ui): add scene extents panel"
```

---

## Task 5: Create Appearance Settings UI Panel

**Files:**
- Modify: `crates/polyscope-ui/src/panels.rs`

**Step 1: Add AppearanceSettings struct**

Add after `SceneExtents`:

```rust
/// Appearance settings for UI.
#[derive(Debug, Clone)]
pub struct AppearanceSettings {
    /// Transparency mode (0=None, 1=Simple, 2=WeightedBlended)
    pub transparency_mode: u32,
    /// SSAA factor (1, 2, or 4)
    pub ssaa_factor: u32,
    /// Max FPS (0 = unlimited)
    pub max_fps: u32,
}

impl Default for AppearanceSettings {
    fn default() -> Self {
        Self {
            transparency_mode: 1, // Simple
            ssaa_factor: 1,
            max_fps: 60,
        }
    }
}
```

**Step 2: Add build_appearance_section function**

Add after `build_scene_extents_section`:

```rust
/// Builds the appearance settings section.
/// Returns true if any setting changed.
pub fn build_appearance_section(ui: &mut Ui, settings: &mut AppearanceSettings) -> bool {
    let mut changed = false;

    CollapsingHeader::new("Appearance")
        .default_open(false)
        .show(ui, |ui| {
            // Transparency mode
            egui::ComboBox::from_label("Transparency")
                .selected_text(match settings.transparency_mode {
                    0 => "None",
                    1 => "Simple",
                    _ => "Weighted Blended",
                })
                .show_ui(ui, |ui| {
                    if ui.selectable_value(&mut settings.transparency_mode, 0, "None").changed() {
                        changed = true;
                    }
                    if ui.selectable_value(&mut settings.transparency_mode, 1, "Simple").changed() {
                        changed = true;
                    }
                    if ui.selectable_value(&mut settings.transparency_mode, 2, "Weighted Blended").changed() {
                        changed = true;
                    }
                });

            ui.separator();

            // SSAA factor
            egui::ComboBox::from_label("Anti-aliasing")
                .selected_text(format!("{}x SSAA", settings.ssaa_factor))
                .show_ui(ui, |ui| {
                    if ui.selectable_value(&mut settings.ssaa_factor, 1, "1x (Off)").changed() {
                        changed = true;
                    }
                    if ui.selectable_value(&mut settings.ssaa_factor, 2, "2x SSAA").changed() {
                        changed = true;
                    }
                    if ui.selectable_value(&mut settings.ssaa_factor, 4, "4x SSAA").changed() {
                        changed = true;
                    }
                });

            ui.separator();

            // Max FPS
            ui.horizontal(|ui| {
                ui.label("Max FPS:");
                let mut fps = settings.max_fps as i32;
                if ui.add(DragValue::new(&mut fps).range(0..=240)).changed() {
                    settings.max_fps = fps.max(0) as u32;
                    changed = true;
                }
                if settings.max_fps == 0 {
                    ui.label("(unlimited)");
                }
            });
        });

    changed
}
```

**Step 3: Export AppearanceSettings from lib.rs**

Update `crates/polyscope-ui/src/lib.rs`:

```rust
pub use panels::{CameraSettings, SceneExtents, AppearanceSettings};
```

**Step 4: Run compilation check**

Run: `cargo check -p polyscope-ui`
Expected: No errors

**Step 5: Commit**

```bash
git add crates/polyscope-ui/src/panels.rs crates/polyscope-ui/src/lib.rs
git commit -m "feat(ui): add appearance settings panel"
```

---

## Task 6: Integrate New Panels into Main Polyscope Crate

**Files:**
- Modify: `crates/polyscope/src/lib.rs`

**Step 1: Re-export new UI types**

Find the UI re-exports section in `crates/polyscope/src/lib.rs` and add:

```rust
pub use polyscope_ui::{CameraSettings, SceneExtents, AppearanceSettings};
```

**Step 2: Add helper functions to sync UI state with backend**

Add these functions to `crates/polyscope/src/lib.rs`:

```rust
/// Syncs CameraSettings from UI to the actual Camera.
pub fn apply_camera_settings(camera: &mut polyscope_render::Camera, settings: &polyscope_ui::CameraSettings) {
    use polyscope_render::{NavigationStyle, ProjectionMode, AxisDirection};

    camera.navigation_style = match settings.navigation_style {
        0 => NavigationStyle::Turntable,
        1 => NavigationStyle::Free,
        2 => NavigationStyle::Planar,
        3 => NavigationStyle::FirstPerson,
        _ => NavigationStyle::None,
    };

    camera.projection_mode = match settings.projection_mode {
        0 => ProjectionMode::Perspective,
        _ => ProjectionMode::Orthographic,
    };

    camera.set_up_direction(match settings.up_direction {
        0 => AxisDirection::PosX,
        1 => AxisDirection::NegX,
        2 => AxisDirection::PosY,
        3 => AxisDirection::NegY,
        4 => AxisDirection::PosZ,
        _ => AxisDirection::NegZ,
    });

    camera.front_direction = match settings.front_direction {
        0 => AxisDirection::PosX,
        1 => AxisDirection::NegX,
        2 => AxisDirection::PosY,
        3 => AxisDirection::NegY,
        4 => AxisDirection::PosZ,
        _ => AxisDirection::NegZ,
    };

    camera.set_fov_degrees(settings.fov_degrees);
    camera.set_near(settings.near);
    camera.set_far(settings.far);
    camera.set_move_speed(settings.move_speed);
    camera.set_ortho_scale(settings.ortho_scale);
}

/// Creates CameraSettings from the current Camera state.
pub fn camera_to_settings(camera: &polyscope_render::Camera) -> polyscope_ui::CameraSettings {
    use polyscope_render::{NavigationStyle, ProjectionMode, AxisDirection};

    polyscope_ui::CameraSettings {
        navigation_style: match camera.navigation_style {
            NavigationStyle::Turntable => 0,
            NavigationStyle::Free => 1,
            NavigationStyle::Planar => 2,
            NavigationStyle::FirstPerson => 3,
            NavigationStyle::None => 4,
        },
        projection_mode: match camera.projection_mode {
            ProjectionMode::Perspective => 0,
            ProjectionMode::Orthographic => 1,
        },
        up_direction: match camera.up_direction {
            AxisDirection::PosX => 0,
            AxisDirection::NegX => 1,
            AxisDirection::PosY => 2,
            AxisDirection::NegY => 3,
            AxisDirection::PosZ => 4,
            AxisDirection::NegZ => 5,
        },
        front_direction: match camera.front_direction {
            AxisDirection::PosX => 0,
            AxisDirection::NegX => 1,
            AxisDirection::PosY => 2,
            AxisDirection::NegY => 3,
            AxisDirection::PosZ => 4,
            AxisDirection::NegZ => 5,
        },
        fov_degrees: camera.fov_degrees(),
        near: camera.near,
        far: camera.far,
        move_speed: camera.move_speed,
        ortho_scale: camera.ortho_scale,
    }
}

/// Gets scene extents from the global context.
pub fn get_scene_extents() -> polyscope_ui::SceneExtents {
    polyscope_core::state::with_context(|ctx| {
        polyscope_ui::SceneExtents {
            auto_compute: ctx.options.auto_compute_scene_extents,
            length_scale: ctx.length_scale,
            bbox_min: ctx.bounding_box.0.to_array(),
            bbox_max: ctx.bounding_box.1.to_array(),
        }
    })
}

/// Sets auto-compute scene extents option.
pub fn set_auto_compute_extents(auto: bool) {
    polyscope_core::state::with_context_mut(|ctx| {
        ctx.options.auto_compute_scene_extents = auto;
    });
}
```

**Step 3: Run compilation check**

Run: `cargo check -p polyscope`
Expected: No errors

**Step 4: Commit**

```bash
git add crates/polyscope/src/lib.rs
git commit -m "feat: add camera and scene settings sync functions"
```

---

## Task 7: Run Full Test Suite and Verify

**Step 1: Run all tests**

Run: `cargo test --workspace`
Expected: All tests pass

**Step 2: Run clippy**

Run: `cargo clippy --workspace`
Expected: No warnings

**Step 3: Format code**

Run: `cargo fmt --all`
Expected: No changes needed (or apply changes)

**Step 4: Final commit if any formatting changes**

```bash
git add -A
git commit -m "chore: format code"
```

---

## Summary

This plan adds the following UI features matching the original C++ Polyscope:

1. **Camera Settings Panel**
   - Navigation style (Turntable, Free, Planar, FirstPerson, None)
   - Projection mode (Perspective, Orthographic)
   - Up/Front direction selectors
   - FOV slider (perspective) / Scale (orthographic)
   - Near/Far clip planes
   - Movement speed

2. **Scene Extents Panel**
   - Auto-compute toggle
   - Length scale display
   - Bounding box display
   - Center display

3. **Appearance Panel**
   - Transparency mode selector
   - SSAA factor selector
   - Max FPS control

4. **Backend Support**
   - Camera enums (NavigationStyle, ProjectionMode, AxisDirection)
   - Orthographic projection support
   - Sync functions between UI and backend
