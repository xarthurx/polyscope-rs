# Phase 4: UI Integration Design

**Date**: 2026-01-21
**Status**: Approved
**Purpose**: Integrate egui for structure/quantity controls and picking system

---

## Overview

Phase 4 adds the user interface layer to polyscope-rs, enabling interactive control of structures and quantities, and element selection via mouse picking.

## Technology Choice: egui

We use **egui** instead of dear-imgui-rs (see `docs/architecture-differences.md` for rationale):

- Pure Rust, no native dependencies
- `cargo build` works without libclang
- WebAssembly ready
- Modern Rust API

**Dependencies:**
```toml
egui = "0.31"
egui-wgpu = "0.31"
egui-winit = "0.31"
```

---

## Architecture

### Integration Points

```
App (winit event loop)
  ├── egui_winit::State       # Handles input events
  ├── egui_wgpu::Renderer     # Renders egui to wgpu
  └── egui::Context           # UI state & building

Render loop:
  1. egui_winit processes winit events
  2. egui::Context::run() builds UI
  3. Render 3D scene (point clouds, etc.)
  4. egui_wgpu renders UI overlay
```

### Panel Layout

```
┌─────────────────────────────────────────────────────────────────┐
│                         Window                                   │
├────────────────┬────────────────────────────────┬───────────────┤
│  Left Panel    │                                │  Right Panel  │
│  (~305px)      │      3D Viewport               │  (~300px)     │
│                │                                │  (when pick   │
│ ┌────────────┐ │                                │   active)     │
│ │ Polyscope  │ │                                │               │
│ │ Controls   │ │    Point clouds, meshes,       │ ┌───────────┐ │
│ │ - View     │ │    vectors rendered here       │ │ Selection │ │
│ │ - Appear.  │ │                                │ │           │ │
│ └────────────┘ │                                │ │ Point #42 │ │
│                │                                │ │ (1.0,2.0) │ │
│ ┌────────────┐ │                                │ │           │ │
│ │ Structures │ │                                │ │ Quantities│ │
│ │            │ │                                │ └───────────┘ │
│ │ ▼ PointCloud│ │                                │               │
│ │   ☑ cloud1 │ │                                │               │
│ │     ▼ Qty  │ │                                │               │
│ │       scalar│ │                                │               │
│ │       color │ │                                │               │
│ │   ☑ cloud2 │ │                                │               │
│ └────────────┘ │                                │               │
└────────────────┴────────────────────────────────┴───────────────┘
```

---

## Components

### 1. Left Panel - Polyscope Controls

Collapsible sections for global settings:

- **View**: Reset view button, background color
- **Appearance**: (future) lighting, materials

### 2. Left Panel - Structure Tree

Hierarchical list of all registered structures:

```rust
CollapsingHeader::new(format!("PointCloud ({})", count))
    .default_open(count <= 8)
    .show(ui, |ui| {
        for (name, structure) in point_clouds {
            ui.horizontal(|ui| {
                // Enable checkbox
                let mut enabled = structure.is_enabled();
                if ui.checkbox(&mut enabled, "").changed() {
                    structure.set_enabled(enabled);
                }

                // Structure name (clickable to expand)
                CollapsingHeader::new(name)
                    .show(ui, |ui| {
                        structure.build_ui(ui);
                        structure.build_quantities_ui(ui);
                    });

                // Options button
                if ui.button("⋮").clicked() {
                    // Show popup menu
                }
            });
        }
    });
```

### 3. Structure-Specific UI

Each structure type implements `build_ui()`:

**PointCloud:**
- Point count display
- Base color picker
- Point radius slider

**SurfaceMesh:**
- Vertex/face count
- Surface color picker
- Shade style dropdown

### 4. Quantity UI

**Scalar Quantity:**
```rust
fn build_ui(&self, ui: &mut egui::Ui) {
    // Colormap selector dropdown
    egui::ComboBox::from_label("Colormap")
        .selected_text(&self.colormap_name)
        .show_ui(ui, |ui| {
            for name in ["viridis", "blues", "reds", "coolwarm", "rainbow"] {
                ui.selectable_value(&mut self.colormap_name, name.to_string(), name);
            }
        });

    // Range controls
    ui.horizontal(|ui| {
        ui.label("Range:");
        ui.add(egui::DragValue::new(&mut self.range_min).speed(0.01));
        ui.label("to");
        ui.add(egui::DragValue::new(&mut self.range_max).speed(0.01));
        if ui.button("Reset").clicked() {
            self.reset_range();
        }
    });
}
```

**Vector Quantity:**
- Length scale slider
- Arrow radius slider
- Color picker

**Color Quantity:**
- Minimal UI (colors are direct)

### 5. Picking System

**Pick buffer rendering:**
- Offscreen framebuffer with unique color per element
- Color encodes structure + element index

**Pick result:**
```rust
pub struct PickResult {
    pub hit: bool,
    pub structure_type: String,
    pub structure_name: String,
    pub element_index: u64,
    pub screen_pos: Vec2,
    pub world_pos: Vec3,
    pub depth: f32,
}
```

**Interaction:**
- Left click: Select element at cursor
- Right click: Clear selection

**Color encoding:**
```rust
fn index_to_pick_color(index: u64) -> [f32; 3] {
    let r = ((index >> 16) & 0xFF) as f32 / 255.0;
    let g = ((index >> 8) & 0xFF) as f32 / 255.0;
    let b = (index & 0xFF) as f32 / 255.0;
    [r, g, b]
}
```

### 6. Right Panel - Selection

Only shown when selection exists:

```rust
fn build_selection_ui(&self, ui: &mut egui::Ui, pick: &PickResult) {
    ui.heading("Selection");
    ui.separator();

    ui.label(format!("Screen: ({:.0}, {:.0})", pick.screen_pos.x, pick.screen_pos.y));
    ui.label(format!("World: ({:.3}, {:.3}, {:.3})",
        pick.world_pos.x, pick.world_pos.y, pick.world_pos.z));

    ui.separator();
    ui.label(format!("{}: {}", pick.structure_type, pick.structure_name));
    ui.label(format!("Element #{}", pick.element_index));

    // Structure-specific pick UI shows quantity values at this element
    structure.build_pick_ui(ui, pick);
}
```

---

## Scope

**Included in Phase 4:**
- egui integration with wgpu render loop
- Left panel (controls + structure tree)
- Structure UI (PointCloud, SurfaceMesh)
- Quantity UI (scalar, vector, color)
- Pick buffer rendering
- Selection panel

**Not included (future phases):**
- Transform gizmos
- Slice planes
- Ground plane with shadows
- Screenshot export UI
- Preferences persistence
- Keyboard shortcuts panel

---

## Files

| File | Action |
|------|--------|
| `Cargo.toml` | Replace dear-imgui deps with egui |
| `crates/polyscope-ui/Cargo.toml` | Rewrite deps for egui |
| `crates/polyscope-ui/src/lib.rs` | egui panel builders |
| `crates/polyscope-ui/src/structure_ui.rs` | Structure tree |
| `crates/polyscope-ui/src/quantity_ui.rs` | Quantity controls |
| `crates/polyscope-ui/src/pick_ui.rs` | Selection panel |
| `crates/polyscope/src/app.rs` | Integrate egui into loop |
| `crates/polyscope-render/src/pick.rs` | Pick buffer rendering |
| `crates/polyscope-core/src/pick.rs` | PickResult struct |
