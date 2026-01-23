# Slice Planes UI Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add UI panel for managing slice planes, allowing users to add, remove, and configure slice planes through the GUI.

**Architecture:** Create UI data structures and panel builders in polyscope-ui that interact with the existing SlicePlane backend in polyscope-core. The UI displays a list of slice planes with per-plane controls for origin, normal, color, transparency, and visibility options.

**Tech Stack:** Rust, egui, glam

---

## Task 1: Add SlicePlaneSettings Struct to UI Crate

**Files:**
- Modify: `crates/polyscope-ui/src/panels.rs:1-77` (add after AppearanceSettings)

**Step 1: Add SlicePlaneSettings struct**

Add after line 76 (after `AppearanceSettings` Default impl) in `crates/polyscope-ui/src/panels.rs`:

```rust
/// Settings for a single slice plane in the UI.
#[derive(Debug, Clone)]
pub struct SlicePlaneSettings {
    /// Name of the slice plane.
    pub name: String,
    /// Whether the slice plane is enabled.
    pub enabled: bool,
    /// Origin point (x, y, z).
    pub origin: [f32; 3],
    /// Normal direction (x, y, z).
    pub normal: [f32; 3],
    /// Whether to draw the plane visualization.
    pub draw_plane: bool,
    /// Whether to draw the widget.
    pub draw_widget: bool,
    /// Color of the plane (r, g, b).
    pub color: [f32; 3],
    /// Transparency (0.0 = transparent, 1.0 = opaque).
    pub transparency: f32,
}

impl Default for SlicePlaneSettings {
    fn default() -> Self {
        Self {
            name: String::new(),
            enabled: true,
            origin: [0.0, 0.0, 0.0],
            normal: [0.0, 1.0, 0.0],
            draw_plane: true,
            draw_widget: true,
            color: [0.5, 0.5, 0.5],
            transparency: 0.3,
        }
    }
}

impl SlicePlaneSettings {
    /// Creates settings with a name.
    pub fn with_name(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            ..Default::default()
        }
    }
}
```

**Step 2: Run compilation check**

Run: `cargo check -p polyscope-ui`
Expected: No errors

**Step 3: Commit**

```bash
git add crates/polyscope-ui/src/panels.rs
git commit -m "feat(ui): add SlicePlaneSettings struct"
```

---

## Task 2: Add Single Slice Plane UI Builder

**Files:**
- Modify: `crates/polyscope-ui/src/panels.rs` (add function after build_appearance_section)

**Step 1: Add build_slice_plane_item function**

Add after `build_appearance_section` function (around line 407):

```rust
/// Builds UI for a single slice plane.
/// Returns true if any setting changed.
fn build_slice_plane_item(ui: &mut Ui, settings: &mut SlicePlaneSettings) -> bool {
    let mut changed = false;

    // Enabled checkbox
    ui.horizontal(|ui| {
        if ui.checkbox(&mut settings.enabled, "Enabled").changed() {
            changed = true;
        }
    });

    ui.separator();

    // Origin
    ui.label("Origin:");
    ui.horizontal(|ui| {
        ui.label("X:");
        if ui.add(DragValue::new(&mut settings.origin[0]).speed(0.1)).changed() {
            changed = true;
        }
        ui.label("Y:");
        if ui.add(DragValue::new(&mut settings.origin[1]).speed(0.1)).changed() {
            changed = true;
        }
        ui.label("Z:");
        if ui.add(DragValue::new(&mut settings.origin[2]).speed(0.1)).changed() {
            changed = true;
        }
    });

    // Normal direction with preset buttons
    ui.label("Normal:");
    ui.horizontal(|ui| {
        if ui.button("+X").clicked() {
            settings.normal = [1.0, 0.0, 0.0];
            changed = true;
        }
        if ui.button("-X").clicked() {
            settings.normal = [-1.0, 0.0, 0.0];
            changed = true;
        }
        if ui.button("+Y").clicked() {
            settings.normal = [0.0, 1.0, 0.0];
            changed = true;
        }
        if ui.button("-Y").clicked() {
            settings.normal = [0.0, -1.0, 0.0];
            changed = true;
        }
        if ui.button("+Z").clicked() {
            settings.normal = [0.0, 0.0, 1.0];
            changed = true;
        }
        if ui.button("-Z").clicked() {
            settings.normal = [0.0, 0.0, -1.0];
            changed = true;
        }
    });

    // Custom normal input
    ui.horizontal(|ui| {
        ui.label("X:");
        if ui.add(DragValue::new(&mut settings.normal[0]).speed(0.01).range(-1.0..=1.0)).changed() {
            changed = true;
        }
        ui.label("Y:");
        if ui.add(DragValue::new(&mut settings.normal[1]).speed(0.01).range(-1.0..=1.0)).changed() {
            changed = true;
        }
        ui.label("Z:");
        if ui.add(DragValue::new(&mut settings.normal[2]).speed(0.01).range(-1.0..=1.0)).changed() {
            changed = true;
        }
    });

    ui.separator();

    // Visualization options
    ui.horizontal(|ui| {
        if ui.checkbox(&mut settings.draw_plane, "Draw plane").changed() {
            changed = true;
        }
        if ui.checkbox(&mut settings.draw_widget, "Draw widget").changed() {
            changed = true;
        }
    });

    // Color
    ui.horizontal(|ui| {
        ui.label("Color:");
        if ui.color_edit_button_rgb(&mut settings.color).changed() {
            changed = true;
        }
    });

    // Transparency
    ui.horizontal(|ui| {
        ui.label("Opacity:");
        if ui.add(Slider::new(&mut settings.transparency, 0.0..=1.0)).changed() {
            changed = true;
        }
    });

    changed
}
```

**Step 2: Run compilation check**

Run: `cargo check -p polyscope-ui`
Expected: No errors

**Step 3: Commit**

```bash
git add crates/polyscope-ui/src/panels.rs
git commit -m "feat(ui): add single slice plane item builder"
```

---

## Task 3: Add Slice Planes Section Builder

**Files:**
- Modify: `crates/polyscope-ui/src/panels.rs` (add main section function)

**Step 1: Add SlicePlanesAction enum for return values**

Add before `build_slice_plane_item`:

```rust
/// Actions that can be triggered from the slice planes UI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SlicePlanesAction {
    /// No action.
    None,
    /// Add a new slice plane with the given name.
    Add(String),
    /// Remove slice plane at the given index.
    Remove(usize),
    /// Settings for a plane were modified.
    Modified(usize),
}
```

**Step 2: Add build_slice_planes_section function**

Add after `build_slice_plane_item`:

```rust
/// Builds the slice planes section.
/// Returns an action if one was triggered (add, remove, or modify).
pub fn build_slice_planes_section(
    ui: &mut Ui,
    planes: &mut Vec<SlicePlaneSettings>,
    new_plane_name: &mut String,
) -> SlicePlanesAction {
    let mut action = SlicePlanesAction::None;

    CollapsingHeader::new("Slice Planes")
        .default_open(false)
        .show(ui, |ui| {
            // Add new plane controls
            ui.horizontal(|ui| {
                ui.label("New plane:");
                ui.text_edit_singleline(new_plane_name);
                if ui.button("Add").clicked() && !new_plane_name.is_empty() {
                    action = SlicePlanesAction::Add(new_plane_name.clone());
                }
            });

            if planes.is_empty() {
                ui.label("No slice planes");
                return;
            }

            ui.separator();

            // List existing planes
            let mut remove_idx = None;
            for (idx, plane) in planes.iter_mut().enumerate() {
                let header_text = format!(
                    "{} {}",
                    if plane.enabled { "●" } else { "○" },
                    plane.name
                );

                CollapsingHeader::new(header_text)
                    .id_salt(format!("slice_plane_{}", idx))
                    .default_open(false)
                    .show(ui, |ui| {
                        if build_slice_plane_item(ui, plane) {
                            if action == SlicePlanesAction::None {
                                action = SlicePlanesAction::Modified(idx);
                            }
                        }

                        ui.separator();
                        if ui.button("Remove").clicked() {
                            remove_idx = Some(idx);
                        }
                    });
            }

            if let Some(idx) = remove_idx {
                action = SlicePlanesAction::Remove(idx);
            }
        });

    action
}
```

**Step 3: Run compilation check**

Run: `cargo check -p polyscope-ui`
Expected: No errors

**Step 4: Commit**

```bash
git add crates/polyscope-ui/src/panels.rs
git commit -m "feat(ui): add slice planes section builder"
```

---

## Task 4: Export New Types from UI Crate

**Files:**
- Modify: `crates/polyscope-ui/src/lib.rs`

**Step 1: Add exports**

Update `crates/polyscope-ui/src/lib.rs` to export the new types:

```rust
pub use panels::{
    AppearanceSettings, CameraSettings, SceneExtents, SlicePlaneSettings, SlicePlanesAction,
};
```

**Step 2: Run compilation check**

Run: `cargo check -p polyscope-ui`
Expected: No errors

**Step 3: Commit**

```bash
git add crates/polyscope-ui/src/lib.rs
git commit -m "feat(ui): export slice plane UI types"
```

---

## Task 5: Add Slice Plane Sync Functions to Main Crate

**Files:**
- Modify: `crates/polyscope/src/lib.rs` (add after camera sync functions around line 1590)

**Step 1: Add re-exports**

Find the UI re-exports line and update to include new types:

```rust
pub use polyscope_ui::{
    AppearanceSettings, CameraSettings, SceneExtents, SlicePlaneSettings, SlicePlanesAction,
};
```

**Step 2: Add sync functions**

Add after `set_auto_compute_extents` function (around line 1590):

```rust
// ============================================================================
// Slice Plane UI Sync Functions
// ============================================================================

/// Gets all slice planes as UI settings.
pub fn get_slice_plane_settings() -> Vec<polyscope_ui::SlicePlaneSettings> {
    with_context(|ctx| {
        ctx.slice_planes
            .values()
            .map(|plane| polyscope_ui::SlicePlaneSettings {
                name: plane.name().to_string(),
                enabled: plane.is_enabled(),
                origin: plane.origin().to_array(),
                normal: plane.normal().to_array(),
                draw_plane: plane.draw_plane(),
                draw_widget: plane.draw_widget(),
                color: plane.color().to_array(),
                transparency: plane.transparency(),
            })
            .collect()
    })
}

/// Applies UI settings to a slice plane.
pub fn apply_slice_plane_settings(settings: &polyscope_ui::SlicePlaneSettings) {
    with_context_mut(|ctx| {
        if let Some(plane) = ctx.get_slice_plane_mut(&settings.name) {
            plane.set_enabled(settings.enabled);
            plane.set_origin(Vec3::from_array(settings.origin));
            plane.set_normal(Vec3::from_array(settings.normal));
            plane.set_draw_plane(settings.draw_plane);
            plane.set_draw_widget(settings.draw_widget);
            plane.set_color(Vec3::from_array(settings.color));
            plane.set_transparency(settings.transparency);
        }
    });
}

/// Handles a slice plane UI action.
/// Returns the new list of settings after the action.
pub fn handle_slice_plane_action(
    action: polyscope_ui::SlicePlanesAction,
    current_settings: &mut Vec<polyscope_ui::SlicePlaneSettings>,
) {
    match action {
        polyscope_ui::SlicePlanesAction::None => {}
        polyscope_ui::SlicePlanesAction::Add(name) => {
            add_slice_plane(&name);
            current_settings.push(polyscope_ui::SlicePlaneSettings::with_name(&name));
        }
        polyscope_ui::SlicePlanesAction::Remove(idx) => {
            if idx < current_settings.len() {
                let name = &current_settings[idx].name;
                remove_slice_plane(name);
                current_settings.remove(idx);
            }
        }
        polyscope_ui::SlicePlanesAction::Modified(idx) => {
            if idx < current_settings.len() {
                apply_slice_plane_settings(&current_settings[idx]);
            }
        }
    }
}
```

**Step 3: Run compilation check**

Run: `cargo check -p polyscope`
Expected: No errors

**Step 4: Commit**

```bash
git add crates/polyscope/src/lib.rs
git commit -m "feat: add slice plane UI sync functions"
```

---

## Task 6: Add Tests for Slice Plane UI Sync

**Files:**
- Modify: `crates/polyscope/src/lib.rs` (add tests at end of test module)

**Step 1: Add tests**

Add to the tests module at the end of `crates/polyscope/src/lib.rs`:

```rust
    #[test]
    fn test_get_slice_plane_settings() {
        setup();
        let name = unique_name("ui_slice_plane");

        // Add a slice plane
        add_slice_plane_with_pose(&name, Vec3::new(1.0, 2.0, 3.0), Vec3::X);

        // Get settings
        let settings = get_slice_plane_settings();
        let found = settings.iter().find(|s| s.name == name);
        assert!(found.is_some());

        let s = found.unwrap();
        assert_eq!(s.origin, [1.0, 2.0, 3.0]);
        assert_eq!(s.normal, [1.0, 0.0, 0.0]);
        assert!(s.enabled);
    }

    #[test]
    fn test_apply_slice_plane_settings() {
        setup();
        let name = unique_name("apply_slice_plane");

        // Add a slice plane
        add_slice_plane(&name);

        // Create modified settings
        let settings = polyscope_ui::SlicePlaneSettings {
            name: name.clone(),
            enabled: false,
            origin: [5.0, 6.0, 7.0],
            normal: [0.0, 0.0, 1.0],
            draw_plane: false,
            draw_widget: true,
            color: [1.0, 0.0, 0.0],
            transparency: 0.8,
        };

        // Apply settings
        apply_slice_plane_settings(&settings);

        // Verify
        let handle = get_slice_plane(&name).unwrap();
        assert!(!handle.is_enabled());
        assert_eq!(handle.origin(), Vec3::new(5.0, 6.0, 7.0));
        assert_eq!(handle.normal(), Vec3::Z);
        assert!(!handle.draw_plane());
        assert!(handle.draw_widget());
        assert_eq!(handle.color(), Vec3::new(1.0, 0.0, 0.0));
        assert!((handle.transparency() - 0.8).abs() < 0.001);
    }

    #[test]
    fn test_handle_slice_plane_action_add() {
        setup();
        let name = unique_name("action_add_plane");
        let mut settings = Vec::new();

        handle_slice_plane_action(
            polyscope_ui::SlicePlanesAction::Add(name.clone()),
            &mut settings,
        );

        assert_eq!(settings.len(), 1);
        assert_eq!(settings[0].name, name);
        assert!(get_slice_plane(&name).is_some());
    }

    #[test]
    fn test_handle_slice_plane_action_remove() {
        setup();
        let name = unique_name("action_remove_plane");

        // Add plane
        add_slice_plane(&name);
        let mut settings = vec![polyscope_ui::SlicePlaneSettings::with_name(&name)];

        // Remove via action
        handle_slice_plane_action(
            polyscope_ui::SlicePlanesAction::Remove(0),
            &mut settings,
        );

        assert!(settings.is_empty());
        assert!(get_slice_plane(&name).is_none());
    }
```

**Step 2: Run tests**

Run: `cargo test -p polyscope -- slice_plane`
Expected: All tests pass

**Step 3: Commit**

```bash
git add crates/polyscope/src/lib.rs
git commit -m "test: add slice plane UI sync tests"
```

---

## Task 7: Wire Slice Planes Panel into App

**Files:**
- Modify: `crates/polyscope/src/app.rs` (find UI building section)

**Step 1: Read current app.rs to find UI integration point**

First, read `crates/polyscope/src/app.rs` to understand the current structure and where panels are built.

**Step 2: Add slice plane UI state to app state**

Find the app state struct and add:

```rust
    /// Slice plane UI settings.
    slice_plane_settings: Vec<polyscope_ui::SlicePlaneSettings>,
    /// New slice plane name input.
    new_slice_plane_name: String,
```

**Step 3: Initialize slice plane state**

In the app initialization, add:

```rust
    slice_plane_settings: crate::get_slice_plane_settings(),
    new_slice_plane_name: String::new(),
```

**Step 4: Add slice planes section to UI building**

Find where other panels are built (Camera, Appearance, etc.) and add:

```rust
    // Slice Planes section
    let slice_action = polyscope_ui::panels::build_slice_planes_section(
        ui,
        &mut self.slice_plane_settings,
        &mut self.new_slice_plane_name,
    );
    if slice_action != polyscope_ui::SlicePlanesAction::None {
        crate::handle_slice_plane_action(slice_action, &mut self.slice_plane_settings);
        if matches!(slice_action, polyscope_ui::SlicePlanesAction::Add(_)) {
            self.new_slice_plane_name.clear();
        }
    }
```

**Step 5: Run compilation check**

Run: `cargo check -p polyscope`
Expected: No errors

**Step 6: Commit**

```bash
git add crates/polyscope/src/app.rs
git commit -m "feat: wire slice planes panel into app UI"
```

---

## Task 8: Run Full Test Suite and Verify

**Step 1: Run all tests**

Run: `cargo test --workspace`
Expected: All tests pass

**Step 2: Run clippy**

Run: `cargo clippy --workspace`
Expected: No warnings

**Step 3: Format code**

Run: `cargo fmt --all`
Expected: Code formatted

**Step 4: Final commit if needed**

```bash
git add -A
git commit -m "chore: format code"
```

---

## Summary

This plan adds a Slice Planes UI panel with:

1. **SlicePlaneSettings struct** - UI representation of slice plane state
2. **SlicePlanesAction enum** - Actions (Add, Remove, Modified) from UI
3. **build_slice_plane_item** - UI for single plane (origin, normal presets, visualization options)
4. **build_slice_planes_section** - Main panel with plane list and add/remove controls
5. **Sync functions** - get_slice_plane_settings(), apply_slice_plane_settings(), handle_slice_plane_action()
6. **App integration** - Wired into main UI panel

UI Features:
- Add new slice planes by name
- Remove existing planes
- Enable/disable planes
- Set origin point (X, Y, Z drag values)
- Set normal direction (preset buttons: +X, -X, +Y, -Y, +Z, -Z, plus custom input)
- Toggle plane visualization
- Toggle widget visualization
- Color picker
- Opacity slider
