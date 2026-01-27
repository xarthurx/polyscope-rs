# Tier 3 — Advanced Quantity Types Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Implement the four remaining advanced quantity types — parameterization, intrinsic vectors, one-forms, and floating quantities — to bring polyscope-rs close to full feature parity with C++ Polyscope 2.x.

**Architecture:** Each feature follows the established quantity pattern: define struct → implement Quantity trait → add UI builder → add registration on parent structure → add shader (if needed) → add tests. Parameterization and intrinsic vectors are surface-mesh-only. One-forms are surface-mesh edge-based. Floating quantities are a new screen-space concept not tied to structures.

**Tech Stack:** Rust, wgpu (WGSL shaders), egui, glam

---

## Task 1: Parameterization Quantity — Core Data Types

**Files:**
- Modify: `crates/polyscope-core/src/quantity.rs` — verify `QuantityKind::Parameterization` exists (it does)
- Create: `crates/polyscope-structures/src/surface_mesh/parameterization_quantity.rs`
- Modify: `crates/polyscope-structures/src/surface_mesh/mod.rs` — add `mod parameterization_quantity; pub use parameterization_quantity::*;`

**Step 1: Create parameterization quantity structs**

Create the file `crates/polyscope-structures/src/surface_mesh/parameterization_quantity.rs` with:

```rust
//! Parameterization (UV) quantities for surface meshes.

use glam::{Vec2, Vec3};
use polyscope_core::quantity::{FaceQuantity, Quantity, QuantityKind, VertexQuantity};

/// Visualization style for parameterization quantities.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParamVizStyle {
    /// Two-color checker pattern over UV space.
    Checker,
    /// Two-color grid lines over UV space.
    Grid,
    /// Checkerboard overlay on radial colormap centered at (0,0).
    LocalCheck,
    /// Distance stripes over radial colormap centered at (0,0).
    LocalRad,
}

impl Default for ParamVizStyle {
    fn default() -> Self {
        Self::Checker
    }
}

/// How to interpret UV coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ParamCoordsType {
    /// Coordinates in [0,1] range.
    #[default]
    Unit,
    /// Coordinates scaled like world-space mesh positions.
    World,
}

/// A vertex parameterization (UV) quantity on a surface mesh.
pub struct MeshVertexParameterizationQuantity {
    name: String,
    structure_name: String,
    coords: Vec<Vec2>,
    enabled: bool,
    // Visualization parameters
    style: ParamVizStyle,
    coords_type: ParamCoordsType,
    checker_size: f32,
    checker_colors: [Vec3; 2],
    grid_line_width: f32,
}

impl MeshVertexParameterizationQuantity {
    /// Creates a new vertex parameterization quantity.
    pub fn new(
        name: impl Into<String>,
        structure_name: impl Into<String>,
        coords: Vec<Vec2>,
    ) -> Self {
        Self {
            name: name.into(),
            structure_name: structure_name.into(),
            coords,
            enabled: false,
            style: ParamVizStyle::default(),
            coords_type: ParamCoordsType::default(),
            checker_size: 0.1,
            checker_colors: [Vec3::new(1.0, 0.4, 0.4), Vec3::new(0.4, 0.4, 1.0)],
            grid_line_width: 0.02,
        }
    }

    #[must_use]
    pub fn coords(&self) -> &[Vec2] { &self.coords }

    #[must_use]
    pub fn style(&self) -> ParamVizStyle { self.style }

    pub fn set_style(&mut self, style: ParamVizStyle) -> &mut Self {
        self.style = style;
        self
    }

    #[must_use]
    pub fn coords_type(&self) -> ParamCoordsType { self.coords_type }

    pub fn set_coords_type(&mut self, ct: ParamCoordsType) -> &mut Self {
        self.coords_type = ct;
        self
    }

    #[must_use]
    pub fn checker_size(&self) -> f32 { self.checker_size }

    pub fn set_checker_size(&mut self, size: f32) -> &mut Self {
        self.checker_size = size;
        self
    }

    #[must_use]
    pub fn checker_colors(&self) -> [Vec3; 2] { self.checker_colors }

    pub fn set_checker_colors(&mut self, colors: [Vec3; 2]) -> &mut Self {
        self.checker_colors = colors;
        self
    }

    /// Compute per-vertex colors based on the current visualization style.
    #[must_use]
    pub fn compute_colors(&self) -> Vec<Vec3> {
        match self.style {
            ParamVizStyle::Checker => self.compute_checker_colors(),
            ParamVizStyle::Grid => self.compute_grid_colors(),
            ParamVizStyle::LocalCheck => self.compute_local_check_colors(),
            ParamVizStyle::LocalRad => self.compute_local_rad_colors(),
        }
    }

    fn compute_checker_colors(&self) -> Vec<Vec3> {
        self.coords.iter().map(|uv| {
            let u_cell = (uv.x / self.checker_size).floor() as i32;
            let v_cell = (uv.y / self.checker_size).floor() as i32;
            if (u_cell + v_cell) % 2 == 0 {
                self.checker_colors[0]
            } else {
                self.checker_colors[1]
            }
        }).collect()
    }

    fn compute_grid_colors(&self) -> Vec<Vec3> {
        self.coords.iter().map(|uv| {
            let u_frac = (uv.x / self.checker_size).fract().abs();
            let v_frac = (uv.y / self.checker_size).fract().abs();
            let on_line = u_frac < self.grid_line_width
                || u_frac > (1.0 - self.grid_line_width)
                || v_frac < self.grid_line_width
                || v_frac > (1.0 - self.grid_line_width);
            if on_line {
                self.checker_colors[1]
            } else {
                self.checker_colors[0]
            }
        }).collect()
    }

    fn compute_local_check_colors(&self) -> Vec<Vec3> {
        self.coords.iter().map(|uv| {
            let r = uv.length();
            let angle = uv.y.atan2(uv.x);
            // Radial colormap hue from angle
            let hue = (angle / std::f32::consts::TAU + 1.0) % 1.0;
            let base = hsv_to_rgb(hue, 0.7, 0.9);
            // Overlay checker
            let u_cell = (uv.x / self.checker_size).floor() as i32;
            let v_cell = (uv.y / self.checker_size).floor() as i32;
            let dim = if (u_cell + v_cell) % 2 == 0 { 1.0 } else { 0.6 };
            base * dim * (1.0 - (-r * 2.0).exp() * 0.5)
        }).collect()
    }

    fn compute_local_rad_colors(&self) -> Vec<Vec3> {
        self.coords.iter().map(|uv| {
            let r = uv.length();
            let angle = uv.y.atan2(uv.x);
            let hue = (angle / std::f32::consts::TAU + 1.0) % 1.0;
            let base = hsv_to_rgb(hue, 0.7, 0.9);
            // Stripe pattern based on radius
            let stripe = ((r / self.checker_size).floor() as i32 % 2 == 0) as u32 as f32;
            let dim = 0.6 + 0.4 * stripe;
            base * dim
        }).collect()
    }

    /// Builds the egui UI for this quantity.
    pub fn build_egui_ui(&mut self, ui: &mut egui::Ui) -> bool {
        polyscope_ui::build_parameterization_quantity_ui(
            ui,
            &self.name,
            &mut self.enabled,
            &mut self.style,
            &mut self.checker_size,
            &mut self.checker_colors,
        )
    }
}

/// Simple HSV to RGB conversion helper.
fn hsv_to_rgb(h: f32, s: f32, v: f32) -> Vec3 {
    let c = v * s;
    let x = c * (1.0 - ((h * 6.0) % 2.0 - 1.0).abs());
    let m = v - c;
    let (r, g, b) = match (h * 6.0) as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    Vec3::new(r + m, g + m, b + m)
}

impl Quantity for MeshVertexParameterizationQuantity {
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
    fn name(&self) -> &str { &self.name }
    fn structure_name(&self) -> &str { &self.structure_name }
    fn kind(&self) -> QuantityKind { QuantityKind::Parameterization }
    fn is_enabled(&self) -> bool { self.enabled }
    fn set_enabled(&mut self, enabled: bool) { self.enabled = enabled; }
    fn build_ui(&mut self, _ui: &dyn std::any::Any) {}
    fn refresh(&mut self) {}
    fn data_size(&self) -> usize { self.coords.len() }
}

impl VertexQuantity for MeshVertexParameterizationQuantity {}

/// A corner (per-face-vertex) parameterization quantity.
/// Used when UV islands are disconnected (different UV at shared vertices).
pub struct MeshCornerParameterizationQuantity {
    name: String,
    structure_name: String,
    coords: Vec<Vec2>, // One per corner (3 * num_triangles for triangle meshes)
    enabled: bool,
    style: ParamVizStyle,
    coords_type: ParamCoordsType,
    checker_size: f32,
    checker_colors: [Vec3; 2],
    grid_line_width: f32,
}

impl MeshCornerParameterizationQuantity {
    /// Creates a new corner parameterization quantity.
    pub fn new(
        name: impl Into<String>,
        structure_name: impl Into<String>,
        coords: Vec<Vec2>,
    ) -> Self {
        Self {
            name: name.into(),
            structure_name: structure_name.into(),
            coords,
            enabled: false,
            style: ParamVizStyle::default(),
            coords_type: ParamCoordsType::default(),
            checker_size: 0.1,
            checker_colors: [Vec3::new(1.0, 0.4, 0.4), Vec3::new(0.4, 0.4, 1.0)],
            grid_line_width: 0.02,
        }
    }

    #[must_use]
    pub fn coords(&self) -> &[Vec2] { &self.coords }

    #[must_use]
    pub fn style(&self) -> ParamVizStyle { self.style }

    pub fn set_style(&mut self, style: ParamVizStyle) -> &mut Self {
        self.style = style;
        self
    }

    #[must_use]
    pub fn checker_size(&self) -> f32 { self.checker_size }

    pub fn set_checker_size(&mut self, size: f32) -> &mut Self {
        self.checker_size = size;
        self
    }

    #[must_use]
    pub fn checker_colors(&self) -> [Vec3; 2] { self.checker_colors }

    pub fn set_checker_colors(&mut self, colors: [Vec3; 2]) -> &mut Self {
        self.checker_colors = colors;
        self
    }

    /// Compute per-corner colors based on the current visualization style.
    /// Returns one color per corner (same length as self.coords).
    #[must_use]
    pub fn compute_colors(&self) -> Vec<Vec3> {
        // Same logic as vertex version, applied per-corner
        self.coords.iter().map(|uv| {
            match self.style {
                ParamVizStyle::Checker => {
                    let u_cell = (uv.x / self.checker_size).floor() as i32;
                    let v_cell = (uv.y / self.checker_size).floor() as i32;
                    if (u_cell + v_cell) % 2 == 0 {
                        self.checker_colors[0]
                    } else {
                        self.checker_colors[1]
                    }
                }
                ParamVizStyle::Grid => {
                    let u_frac = (uv.x / self.checker_size).fract().abs();
                    let v_frac = (uv.y / self.checker_size).fract().abs();
                    let on_line = u_frac < self.grid_line_width
                        || u_frac > (1.0 - self.grid_line_width)
                        || v_frac < self.grid_line_width
                        || v_frac > (1.0 - self.grid_line_width);
                    if on_line { self.checker_colors[1] } else { self.checker_colors[0] }
                }
                ParamVizStyle::LocalCheck => {
                    let angle = uv.y.atan2(uv.x);
                    let hue = (angle / std::f32::consts::TAU + 1.0) % 1.0;
                    let base = hsv_to_rgb(hue, 0.7, 0.9);
                    let u_cell = (uv.x / self.checker_size).floor() as i32;
                    let v_cell = (uv.y / self.checker_size).floor() as i32;
                    let dim = if (u_cell + v_cell) % 2 == 0 { 1.0 } else { 0.6 };
                    let r = uv.length();
                    base * dim * (1.0 - (-r * 2.0).exp() * 0.5)
                }
                ParamVizStyle::LocalRad => {
                    let angle = uv.y.atan2(uv.x);
                    let hue = (angle / std::f32::consts::TAU + 1.0) % 1.0;
                    let base = hsv_to_rgb(hue, 0.7, 0.9);
                    let r = uv.length();
                    let stripe = ((r / self.checker_size).floor() as i32 % 2 == 0) as u32 as f32;
                    let dim = 0.6 + 0.4 * stripe;
                    base * dim
                }
            }
        }).collect()
    }

    /// Builds the egui UI for this quantity.
    pub fn build_egui_ui(&mut self, ui: &mut egui::Ui) -> bool {
        polyscope_ui::build_parameterization_quantity_ui(
            ui,
            &self.name,
            &mut self.enabled,
            &mut self.style,
            &mut self.checker_size,
            &mut self.checker_colors,
        )
    }
}

impl Quantity for MeshCornerParameterizationQuantity {
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
    fn name(&self) -> &str { &self.name }
    fn structure_name(&self) -> &str { &self.structure_name }
    fn kind(&self) -> QuantityKind { QuantityKind::Parameterization }
    fn is_enabled(&self) -> bool { self.enabled }
    fn set_enabled(&mut self, enabled: bool) { self.enabled = enabled; }
    fn build_ui(&mut self, _ui: &dyn std::any::Any) {}
    fn refresh(&mut self) {}
    fn data_size(&self) -> usize { self.coords.len() }
}

impl FaceQuantity for MeshCornerParameterizationQuantity {}
```

**Step 2: Wire up module in surface_mesh/mod.rs**

Add `mod parameterization_quantity;` and `pub use parameterization_quantity::*;` alongside existing quantity module declarations.

**Step 3: Add registration methods on SurfaceMesh**

```rust
pub fn add_vertex_parameterization_quantity(
    &mut self,
    name: impl Into<String>,
    coords: Vec<Vec2>,
) -> &mut Self {
    let quantity = MeshVertexParameterizationQuantity::new(name, self.name.clone(), coords);
    self.add_quantity(Box::new(quantity));
    self
}

pub fn add_corner_parameterization_quantity(
    &mut self,
    name: impl Into<String>,
    coords: Vec<Vec2>,
) -> &mut Self {
    let quantity = MeshCornerParameterizationQuantity::new(name, self.name.clone(), coords);
    self.add_quantity(Box::new(quantity));
    self
}
```

**Step 4: Add UI downcasting in SurfaceMesh::build_egui_ui**

Add cases for `MeshVertexParameterizationQuantity` and `MeshCornerParameterizationQuantity` in the quantity UI rendering loop.

**Step 5: Add tests**

Test construction, getters/setters, compute_colors for all 4 styles, and registration on SurfaceMesh.

**Step 6: Commit**

```bash
git add -A && git commit -m "feat: add parameterization (UV) quantities for surface meshes"
```

---

## Task 2: Parameterization Quantity — UI Builder

**Files:**
- Modify: `crates/polyscope-ui/src/quantity_ui.rs` — add `build_parameterization_quantity_ui()`
- Modify: `crates/polyscope-ui/src/lib.rs` — re-export

**Step 1: Add the UI builder function**

```rust
/// Builds UI for a parameterization quantity.
pub fn build_parameterization_quantity_ui(
    ui: &mut Ui,
    name: &str,
    enabled: &mut bool,
    style: &mut ParamVizStyle,
    checker_size: &mut f32,
    checker_colors: &mut [Vec3; 2],
) -> bool {
    // Checkbox + name
    // ComboBox for style selection (Checker, Grid, LocalCheck, LocalRad)
    // DragValue for checker_size
    // Color pickers for checker_colors[0] and checker_colors[1]
}
```

Note: `ParamVizStyle` is defined in polyscope-structures, so polyscope-ui needs to either:
- (a) Move the enum to polyscope-core, or
- (b) Use a string/int representation in the UI builder

**Recommendation:** Move `ParamVizStyle` and `ParamCoordsType` to `polyscope-core/src/quantity.rs` alongside `QuantityKind` so both polyscope-ui and polyscope-structures can reference them.

**Step 2: Re-export from polyscope-ui/src/lib.rs**

**Step 3: Commit**

```bash
git add -A && git commit -m "feat(ui): add parameterization quantity UI builder"
```

---

## Task 3: Intrinsic Vector Quantities

**Files:**
- Create: `crates/polyscope-structures/src/surface_mesh/intrinsic_vector_quantity.rs`
- Modify: `crates/polyscope-structures/src/surface_mesh/mod.rs`

**Step 1: Create intrinsic vector quantity structs**

Two structs:
- `MeshVertexIntrinsicVectorQuantity` — 2D vectors + tangent basis per vertex
- `MeshFaceIntrinsicVectorQuantity` — 2D vectors + tangent basis per face

```rust
pub struct MeshVertexIntrinsicVectorQuantity {
    name: String,
    structure_name: String,
    vectors: Vec<Vec2>,       // 2D tangent-space vectors
    basis_x: Vec<Vec3>,       // Per-element X axis of tangent frame
    basis_y: Vec<Vec3>,       // Per-element Y axis of tangent frame
    n_sym: u32,               // Symmetry: 1=vector, 2=line, 4=cross
    enabled: bool,
    length_scale: f32,
    radius: f32,
    color: Vec3,
}
```

**Key method — project to world space:**

```rust
/// Project 2D tangent-space vectors to 3D world space.
pub fn compute_world_vectors(&self) -> Vec<Vec3> {
    self.vectors.iter().enumerate().map(|(i, v2d)| {
        self.basis_x[i] * v2d.x + self.basis_y[i] * v2d.y
    }).collect()
}
```

**Symmetry rendering:**

For `n_sym > 1`, generate rotated copies of each vector.

**Step 2: Add default tangent basis computation on SurfaceMesh**

A helper method that computes a default tangent basis from mesh geometry:

```rust
/// Compute default per-face tangent basis from first edge direction.
pub fn compute_face_tangent_basis(&self) -> (Vec<Vec3>, Vec<Vec3>) {
    // For each face: basis_x = normalize(v1 - v0), basis_y = cross(normal, basis_x)
}

/// Compute default per-vertex tangent basis from area-weighted face bases.
pub fn compute_vertex_tangent_basis(&self) -> (Vec<Vec3>, Vec<Vec3>) {
    // Average incident face bases, then orthonormalize
}
```

**Step 3: Registration methods**

```rust
pub fn add_vertex_intrinsic_vector_quantity(
    &mut self,
    name: impl Into<String>,
    vectors: Vec<Vec2>,
    basis_x: Vec<Vec3>,
    basis_y: Vec<Vec3>,
) -> &mut Self

// Convenience version that auto-computes tangent basis
pub fn add_vertex_intrinsic_vector_quantity_auto(
    &mut self,
    name: impl Into<String>,
    vectors: Vec<Vec2>,
) -> &mut Self
```

**Step 4: UI builder in polyscope-ui**

Extend `build_vector_quantity_ui` or create `build_intrinsic_vector_quantity_ui` with an extra `n_sym` control.

**Step 5: Tests**

Test tangent basis computation, world-space projection, symmetry generation.

**Step 6: Commit**

```bash
git add -A && git commit -m "feat: add intrinsic (tangent-space) vector quantities for surface meshes"
```

---

## Task 4: One-Form Quantities

**Files:**
- Create: `crates/polyscope-structures/src/surface_mesh/one_form_quantity.rs`
- Modify: `crates/polyscope-structures/src/surface_mesh/mod.rs`
- Modify: `crates/polyscope-core/src/quantity.rs` — add `QuantityKind::OneForm` (optional, could use `Other`)

**Step 1: Edge infrastructure on SurfaceMesh**

The SurfaceMesh already has `edges()` returning unique edges. Need:
- Edge orientation storage (canonical direction: lower → higher vertex index)
- Edge midpoint computation
- Edge direction vector computation

**Step 2: Create one-form quantity struct**

```rust
pub struct MeshOneFormQuantity {
    name: String,
    structure_name: String,
    values: Vec<f32>,           // One scalar per edge
    orientations: Vec<bool>,    // Edge orientation convention
    enabled: bool,
    length_scale: f32,
    radius: f32,
    color: Vec3,
}
```

**Key methods:**

```rust
/// Convert edge scalars + orientations to vector field for rendering.
/// Returns (positions, vectors) — one arrow per edge at edge midpoint.
pub fn compute_edge_vectors(
    &self,
    vertices: &[Vec3],
    edges: &[[u32; 2]],
) -> (Vec<Vec3>, Vec<Vec3>) {
    // For each edge:
    // direction = normalize(v[e[1]] - v[e[0]])
    // if !orientation[i]: flip direction
    // vector = direction * values[i]
    // position = midpoint of edge
}
```

**Step 3: Registration**

```rust
pub fn add_one_form_quantity(
    &mut self,
    name: impl Into<String>,
    values: Vec<f32>,
    orientations: Vec<bool>,
) -> &mut Self
```

**Step 4: UI builder**

Similar to vector quantity UI — length, radius, color.

**Step 5: Tests**

Test edge vector computation, orientation handling, sign conventions.

**Step 6: Commit**

```bash
git add -A && git commit -m "feat: add one-form quantities for surface meshes"
```

---

## Task 5: Floating Quantities — Scalar Image

**Files:**
- Create: `crates/polyscope-structures/src/floating/mod.rs`
- Create: `crates/polyscope-structures/src/floating/scalar_image.rs`
- Create: `crates/polyscope-structures/src/floating/color_image.rs`
- Modify: `crates/polyscope-structures/src/lib.rs` — add `pub mod floating;`
- Modify: `crates/polyscope/src/lib.rs` — add registration functions and re-exports

**Step 1: Define floating quantity base types**

```rust
/// Image origin convention.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ImageOrigin {
    #[default]
    UpperLeft,
    LowerLeft,
}

/// A floating scalar image quantity (not attached to any structure).
pub struct FloatingScalarImage {
    name: String,
    width: u32,
    height: u32,
    values: Vec<f32>,
    origin: ImageOrigin,
    enabled: bool,
    colormap_name: String,
    data_min: f32,
    data_max: f32,
}

/// A floating color image quantity.
pub struct FloatingColorImage {
    name: String,
    width: u32,
    height: u32,
    colors: Vec<Vec3>,  // RGB per pixel
    origin: ImageOrigin,
    enabled: bool,
}
```

**Step 2: Implement Quantity trait**

Floating quantities implement `Quantity` but NO marker trait (not element-based). Use `QuantityKind::Other`.

**Step 3: Registration in main crate**

```rust
pub fn register_floating_scalar_image(
    name: impl Into<String>,
    width: u32,
    height: u32,
    values: Vec<f32>,
) -> FloatingScalarImageHandle

pub fn register_floating_color_image(
    name: impl Into<String>,
    width: u32,
    height: u32,
    colors: Vec<Vec3>,
) -> FloatingColorImageHandle
```

**Step 4: Storage**

Add a `floating_quantities: Vec<Box<dyn Quantity>>` field to `Context` in polyscope-core.

**Step 5: UI panel**

Floating quantities appear in a separate section of the UI panel (not under any structure).

**Step 6: Tests**

Test construction, image indexing, min/max computation.

**Step 7: Commit**

```bash
git add -A && git commit -m "feat: add floating quantity infrastructure with scalar/color image types"
```

---

## Task 6: Floating Quantities — Render Images (Depth Compositing)

**Files:**
- Create: `crates/polyscope-structures/src/floating/render_image.rs`
- Create: `crates/polyscope-render/src/shaders/floating_image.wgsl` (if GPU compositing needed)

**Step 1: Define render image types**

```rust
/// A depth render image (geometry from external renderer).
pub struct FloatingDepthRenderImage {
    name: String,
    width: u32,
    height: u32,
    depths: Vec<f32>,           // Radial distance from camera
    normals: Option<Vec<Vec3>>, // Optional world-space normals
    origin: ImageOrigin,
    enabled: bool,
}

/// A color render image (colored geometry from external renderer).
pub struct FloatingColorRenderImage {
    name: String,
    width: u32,
    height: u32,
    depths: Vec<f32>,
    colors: Vec<Vec3>,          // Per-pixel RGB
    normals: Option<Vec<Vec3>>,
    origin: ImageOrigin,
    enabled: bool,
}

/// A raw color render image (direct display, no shading).
pub struct FloatingRawColorImage {
    name: String,
    width: u32,
    height: u32,
    colors: Vec<Vec3>,
    origin: ImageOrigin,
    enabled: bool,
}
```

**Step 2: Tests**

Test construction, pixel access, depth compositing helpers.

**Step 3: Commit**

```bash
git add -A && git commit -m "feat: add render image floating quantities (depth, color, raw)"
```

---

## Task 7: Update Examples and Documentation

**Files:**
- Modify: `examples/surface_mesh_demo.rs` — add parameterization and intrinsic vector usage
- Modify: `todo.md` — mark Tier 3 items as done
- Modify: `CLAUDE.md` — update feature lists
- Modify: `docs/architecture-differences.md` — update missing features
- Modify: `README.md` — update feature table

**Step 1: Update surface mesh demo**

Add UV coordinate generation (e.g., spherical projection) and add a parameterization quantity.

**Step 2: Update all docs**

Move items from "Planned" to "Completed" in todo.md, CLAUDE.md, architecture-differences.md, README.md.

**Step 3: Commit**

```bash
git add -A && git commit -m "docs: update examples and documentation for Tier 3 quantities"
```

---

## Task 8: Final Build Verification

**Step 1:** `cargo build`
**Step 2:** `cargo clippy` — zero warnings
**Step 3:** `cargo test` — all pass
**Step 4:** `cargo build --examples` — all examples compile
**Step 5:** `cargo fmt` — clean formatting

**Step 6: Final commit (if any formatting changes)**

```bash
git add -A && git commit -m "chore: final cleanup for Tier 3 quantities"
```

---

## Execution Order Summary

| Task | Feature | Depends On |
|------|---------|-----------|
| 1 | Parameterization — data types | — |
| 2 | Parameterization — UI builder | Task 1 (needs ParamVizStyle) |
| 3 | Intrinsic vectors | — |
| 4 | One-forms | — |
| 5 | Floating quantities — images | — |
| 6 | Floating quantities — render images | Task 5 |
| 7 | Examples and docs | Tasks 1-6 |
| 8 | Final verification | Task 7 |

**Independent tasks (can parallelize):** Tasks 1+3, Tasks 1+4, Tasks 3+4, Tasks 3+5, Tasks 4+5
