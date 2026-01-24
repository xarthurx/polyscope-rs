# Gizmo Rendering Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add visual 3D transformation gizmo rendering using the transform-gizmo-egui crate, enabling interactive translation/rotation/scaling of selected structures.

**Architecture:** Upgrade egui from 0.31 to 0.33, add transform-gizmo-egui dependency, integrate gizmo rendering into the existing egui frame, and connect gizmo interactions to the structure transform system. The gizmo will render when a structure is selected and gizmo is visible.

**Tech Stack:** Rust, egui 0.33, transform-gizmo-egui, wgpu, glam

---

## Task 1: Upgrade egui Dependencies

**Files:**
- Modify: `Cargo.toml`

**Step 1: Update workspace dependencies**

Edit `Cargo.toml` to upgrade egui from 0.31 to 0.33:

```toml
# UI (egui - pure Rust, no native dependencies)
egui = "0.33"
egui-wgpu = "0.33"
egui-winit = "0.33"
```

**Step 2: Run cargo update for egui packages**

Run: `cargo update -p egui -p egui-wgpu -p egui-winit`
Expected: Dependencies updated

**Step 3: Run compilation check**

Run: `cargo check --workspace`
Expected: May have some API changes to fix (proceed to Step 4 if errors)

**Step 4: Fix any egui API changes**

Common changes from 0.31 to 0.33:
- Check if `egui::Context` API changed
- Check if `egui_wgpu::Renderer` API changed
- Check if `egui_winit::State` API changed

Run: `cargo check --workspace` after each fix

**Step 5: Run tests**

Run: `cargo test --workspace`
Expected: All tests pass

**Step 6: Commit**

```bash
git add Cargo.toml Cargo.lock
git commit -m "deps: upgrade egui from 0.31 to 0.33"
```

---

## Task 2: Add transform-gizmo-egui Dependency

**Files:**
- Modify: `Cargo.toml` (workspace)
- Modify: `crates/polyscope-ui/Cargo.toml`

**Step 1: Add to workspace dependencies**

Add to `Cargo.toml` workspace dependencies section:

```toml
transform-gizmo-egui = "0.8"
```

**Step 2: Add to polyscope-ui crate**

Add to `crates/polyscope-ui/Cargo.toml` dependencies:

```toml
transform-gizmo-egui = { workspace = true }
```

**Step 3: Run compilation check**

Run: `cargo check -p polyscope-ui`
Expected: No errors

**Step 4: Commit**

```bash
git add Cargo.toml crates/polyscope-ui/Cargo.toml
git commit -m "deps: add transform-gizmo-egui 0.8"
```

---

## Task 3: Create Gizmo Wrapper Module

**Files:**
- Create: `crates/polyscope-ui/src/gizmo.rs`
- Modify: `crates/polyscope-ui/src/lib.rs`

**Step 1: Create gizmo wrapper module**

Create `crates/polyscope-ui/src/gizmo.rs`:

```rust
//! Visual 3D gizmo integration using transform-gizmo-egui.

use egui::Ui;
use glam::{Mat4, Quat, Vec3};
use transform_gizmo_egui::{Gizmo, GizmoConfig, GizmoMode, GizmoOrientation, GizmoVisuals};

/// Wrapper around transform-gizmo-egui for polyscope integration.
pub struct TransformGizmo {
    gizmo: Gizmo,
}

impl Default for TransformGizmo {
    fn default() -> Self {
        Self::new()
    }
}

impl TransformGizmo {
    /// Creates a new transform gizmo.
    pub fn new() -> Self {
        Self {
            gizmo: Gizmo::default(),
        }
    }

    /// Draws the gizmo and handles interaction.
    ///
    /// Returns the updated transform if the gizmo was manipulated.
    ///
    /// # Arguments
    /// * `ui` - The egui UI context
    /// * `view_matrix` - Camera view matrix
    /// * `projection_matrix` - Camera projection matrix
    /// * `model_matrix` - Current transform of the object
    /// * `mode` - Gizmo mode (0=Translate, 1=Rotate, 2=Scale)
    /// * `space` - Coordinate space (0=World, 1=Local)
    /// * `viewport_size` - Size of the viewport in pixels
    pub fn interact(
        &mut self,
        ui: &mut Ui,
        view_matrix: Mat4,
        projection_matrix: Mat4,
        model_matrix: Mat4,
        mode: u32,
        space: u32,
        viewport_size: [f32; 2],
    ) -> Option<Mat4> {
        let gizmo_mode = match mode {
            0 => GizmoMode::Translate,
            1 => GizmoMode::Rotate,
            2 => GizmoMode::Scale,
            _ => GizmoMode::Translate,
        };

        let orientation = match space {
            0 => GizmoOrientation::Global,
            1 => GizmoOrientation::Local,
            _ => GizmoOrientation::Global,
        };

        // Convert glam matrices to mint types (transform-gizmo uses mint)
        let view: mint::ColumnMatrix4<f32> = view_matrix.into();
        let projection: mint::ColumnMatrix4<f32> = projection_matrix.into();
        let model: mint::ColumnMatrix4<f32> = model_matrix.into();

        let config = GizmoConfig {
            view_matrix: view.into(),
            projection_matrix: projection.into(),
            viewport: egui::Rect::from_min_size(
                egui::pos2(0.0, 0.0),
                egui::vec2(viewport_size[0], viewport_size[1]),
            ),
            mode: gizmo_mode,
            orientation,
            snapping: false,
            snap_distance: 0.0,
            snap_angle: 0.0,
            snap_scale: 0.0,
            visuals: GizmoVisuals::default(),
            ..Default::default()
        };

        // Interact with gizmo
        if let Some((_result, new_transforms)) = self.gizmo.interact(ui, &config, &[model.into()]) {
            if let Some(new_transform) = new_transforms.first() {
                let new_mat: mint::ColumnMatrix4<f32> = (*new_transform).into();
                return Some(Mat4::from_cols_array_2d(&[
                    [new_mat.x.x, new_mat.x.y, new_mat.x.z, new_mat.x.w],
                    [new_mat.y.x, new_mat.y.y, new_mat.y.z, new_mat.y.w],
                    [new_mat.z.x, new_mat.z.y, new_mat.z.z, new_mat.z.w],
                    [new_mat.w.x, new_mat.w.y, new_mat.w.z, new_mat.w.w],
                ]));
            }
        }

        None
    }

    /// Decomposes a Mat4 into translation, rotation (Euler degrees), and scale.
    pub fn decompose_transform(matrix: Mat4) -> (Vec3, Vec3, Vec3) {
        let (scale, rotation, translation) = matrix.to_scale_rotation_translation();
        let euler = rotation.to_euler(glam::EulerRot::XYZ);
        let euler_degrees = Vec3::new(
            euler.0.to_degrees(),
            euler.1.to_degrees(),
            euler.2.to_degrees(),
        );
        (translation, euler_degrees, scale)
    }

    /// Composes a Mat4 from translation, rotation (Euler degrees), and scale.
    pub fn compose_transform(translation: Vec3, euler_degrees: Vec3, scale: Vec3) -> Mat4 {
        let rotation = Quat::from_euler(
            glam::EulerRot::XYZ,
            euler_degrees.x.to_radians(),
            euler_degrees.y.to_radians(),
            euler_degrees.z.to_radians(),
        );
        Mat4::from_scale_rotation_translation(scale, rotation, translation)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gizmo_creation() {
        let gizmo = TransformGizmo::new();
        // Just verify it can be created
        drop(gizmo);
    }

    #[test]
    fn test_decompose_compose_roundtrip() {
        let translation = Vec3::new(1.0, 2.0, 3.0);
        let euler_degrees = Vec3::new(45.0, 30.0, 15.0);
        let scale = Vec3::new(1.0, 2.0, 1.5);

        let matrix = TransformGizmo::compose_transform(translation, euler_degrees, scale);
        let (t, r, s) = TransformGizmo::decompose_transform(matrix);

        assert!((t - translation).length() < 0.001);
        assert!((r - euler_degrees).length() < 0.1);
        assert!((s - scale).length() < 0.001);
    }
}
```

**Step 2: Export from lib.rs**

Add to `crates/polyscope-ui/src/lib.rs`:

```rust
mod gizmo;
pub use gizmo::TransformGizmo;
```

**Step 3: Run compilation check**

Run: `cargo check -p polyscope-ui`
Expected: No errors (may need to adjust based on actual transform-gizmo-egui API)

**Step 4: Run tests**

Run: `cargo test -p polyscope-ui`
Expected: All tests pass

**Step 5: Commit**

```bash
git add crates/polyscope-ui/src/gizmo.rs crates/polyscope-ui/src/lib.rs
git commit -m "feat(ui): add TransformGizmo wrapper for transform-gizmo-egui"
```

---

## Task 4: Add Gizmo State to App

**Files:**
- Modify: `crates/polyscope/src/app.rs`

**Step 1: Add TransformGizmo to App struct**

Find the App struct (around line 25) and add after `selection_info`:

```rust
    // Visual gizmo
    transform_gizmo: polyscope_ui::TransformGizmo,
```

**Step 2: Initialize in App::new()**

In `App::new()` (around line 62), add after `selection_info`:

```rust
            transform_gizmo: polyscope_ui::TransformGizmo::new(),
```

**Step 3: Run compilation check**

Run: `cargo check -p polyscope`
Expected: No errors

**Step 4: Commit**

```bash
git add crates/polyscope/src/app.rs
git commit -m "feat: add TransformGizmo state to App"
```

---

## Task 5: Integrate Gizmo into Render Loop

**Files:**
- Modify: `crates/polyscope/src/app.rs`

**Step 1: Add gizmo rendering in the egui frame**

In the `render()` function, find where `polyscope_ui::build_left_panel` ends (around line 408). Add after the panel closure, before `let egui_output = egui.end_frame(window);`:

```rust
        // Render transform gizmo if visible and something is selected
        if self.gizmo_settings.visible && self.selection_info.has_selection {
            // Get camera matrices from engine
            let view_matrix = engine.camera.view_matrix();
            let projection_matrix = engine.camera.projection_matrix(
                engine.width as f32 / engine.height as f32,
            );

            // Get current transform as matrix
            let current_transform = polyscope_ui::TransformGizmo::compose_transform(
                glam::Vec3::from(self.selection_info.translation),
                glam::Vec3::from(self.selection_info.rotation_degrees),
                glam::Vec3::from(self.selection_info.scale),
            );

            // Create a central panel for the gizmo overlay
            egui::CentralPanel::default()
                .frame(egui::Frame::NONE)
                .show(&egui.context, |ui| {
                    let viewport_size = [engine.width as f32, engine.height as f32];

                    if let Some(new_transform) = self.transform_gizmo.interact(
                        ui,
                        view_matrix,
                        projection_matrix,
                        current_transform,
                        self.gizmo_settings.mode,
                        self.gizmo_settings.space,
                        viewport_size,
                    ) {
                        // Decompose and update selection info
                        let (translation, rotation, scale) =
                            polyscope_ui::TransformGizmo::decompose_transform(new_transform);
                        self.selection_info.translation = translation.into();
                        self.selection_info.rotation_degrees = rotation.into();
                        self.selection_info.scale = scale.into();

                        // Apply to selected structure
                        crate::handle_gizmo_action(
                            polyscope_ui::GizmoAction::TransformChanged,
                            &self.gizmo_settings,
                            &self.selection_info,
                        );
                    }
                });
        }
```

**Step 2: Add glam import if not present**

At the top of `app.rs`, ensure glam is imported:

```rust
use glam;
```

**Step 3: Run compilation check**

Run: `cargo check -p polyscope`
Expected: No errors

**Step 4: Commit**

```bash
git add crates/polyscope/src/app.rs
git commit -m "feat: integrate gizmo rendering into render loop"
```

---

## Task 6: Add Camera Matrix Methods

**Files:**
- Modify: `crates/polyscope-render/src/camera.rs`

**Step 1: Check if view_matrix and projection_matrix exist**

Run: `grep -n "view_matrix\|projection_matrix" crates/polyscope-render/src/camera.rs`

If they don't exist, add them to the Camera struct:

```rust
    /// Returns the view matrix for this camera.
    pub fn view_matrix(&self) -> glam::Mat4 {
        glam::Mat4::look_at_rh(self.position, self.target, self.up)
    }

    /// Returns the projection matrix for this camera.
    pub fn projection_matrix(&self, aspect_ratio: f32) -> glam::Mat4 {
        glam::Mat4::perspective_rh(
            self.fov.to_radians(),
            aspect_ratio,
            self.near,
            self.far,
        )
    }
```

**Step 2: Run compilation check**

Run: `cargo check -p polyscope-render`
Expected: No errors

**Step 3: Commit if changes were made**

```bash
git add crates/polyscope-render/src/camera.rs
git commit -m "feat(render): add view_matrix and projection_matrix to Camera"
```

---

## Task 7: Fix transform-gizmo-egui API Integration

**Files:**
- Modify: `crates/polyscope-ui/src/gizmo.rs`

This task handles any API differences in the actual transform-gizmo-egui crate. The code in Task 3 is based on documentation; actual API may differ.

**Step 1: Check actual API**

Run: `cargo doc -p transform-gizmo-egui --open`

Review the actual types and methods available.

**Step 2: Adjust gizmo.rs to match actual API**

Common adjustments needed:
- Check if `GizmoConfig` fields match
- Check if `Gizmo::interact` signature matches
- Check matrix conversion between glam and mint

**Step 3: Run compilation check**

Run: `cargo check -p polyscope-ui`
Expected: No errors

**Step 4: Run tests**

Run: `cargo test -p polyscope-ui`
Expected: All tests pass

**Step 5: Commit**

```bash
git add crates/polyscope-ui/src/gizmo.rs
git commit -m "fix(ui): adjust gizmo API to match transform-gizmo-egui"
```

---

## Task 8: Run Full Test Suite and Verify

**Step 1: Run all tests**

Run: `cargo test --workspace`
Expected: All tests pass

**Step 2: Run clippy**

Run: `cargo clippy --workspace`
Expected: No warnings (or only acceptable ones)

**Step 3: Format code**

Run: `cargo fmt --all`
Expected: Code formatted

**Step 4: Build and run example**

Run: `cargo run --example basic_demo` (or another example that shows the viewer)
Expected: Gizmo appears when a structure is selected

**Step 5: Final commit if needed**

```bash
git add -A
git commit -m "chore: format code and fix any remaining issues"
```

---

## Summary

This plan adds visual 3D gizmo rendering with:

1. **egui upgrade** - 0.31 → 0.33 for transform-gizmo-egui compatibility
2. **transform-gizmo-egui integration** - Uses battle-tested gizmo library (same approach as C++ Polyscope using ImGuizmo)
3. **TransformGizmo wrapper** - Clean interface between polyscope-ui and external crate
4. **App integration** - Gizmo renders in egui overlay when structure is selected

Gizmo Features:
- Translate mode (arrows along X/Y/Z axes)
- Rotate mode (circles around axes)
- Scale mode (boxes along axes)
- World/Local coordinate space
- Click and drag interaction
- Visual feedback on hover/active

The gizmo appears automatically when:
1. A structure is selected (click on it)
2. Gizmo visibility is enabled in the Transform/Gizmo panel

## Notes

- The transform-gizmo-egui crate handles all the complex gizmo geometry, rendering, and picking
- This mirrors how the original C++ Polyscope uses ImGuizmo
- If transform-gizmo-egui API differs significantly, Task 7 covers adjustments
