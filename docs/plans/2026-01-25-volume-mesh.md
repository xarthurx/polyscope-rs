# Volume Mesh Complete Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Complete the volume mesh implementation with proper interior/exterior face detection, quantities (scalar, color, vector), and slice plane integration.

**Architecture:** The volume mesh renders exterior faces only (faces not shared between cells). Interior faces are detected by hashing sorted face vertices and counting occurrences. Quantities follow the existing VolumeGridScalarQuantity pattern, with separate types for vertex and cell quantities.

**Tech Stack:** wgpu, glam, egui, Rust HashMap for face counting

---

## Current State Analysis

**What exists:**
- Basic VolumeMesh struct with vertices, cells (tet/hex), colors
- Cell type detection (tet if cell[4] == u32::MAX)
- Basic rendering via SurfaceMeshRenderData
- HasQuantities trait implementation (storage only)
- UI with color picker and edge width

**What's missing:**
- Interior/exterior face detection (renders ALL faces, causing overdraw)
- Vertex scalar quantities
- Cell scalar quantities
- Vertex color quantities
- Cell color quantities
- Vertex vector quantities
- Cell vector quantities
- Slice plane integration (level sets)
- Picking support

---

## Task 1: Interior/Exterior Face Detection

**Files:**
- Modify: `crates/polyscope-structures/src/volume_mesh/mod.rs`
- Test: `crates/polyscope-structures/src/volume_mesh/mod.rs` (inline tests)

**Step 1: Write the failing test**

Add to existing test module:

```rust
#[test]
fn test_interior_face_detection() {
    // Two tets sharing a face
    let vertices = vec![
        Vec3::new(0.0, 0.0, 0.0),  // 0
        Vec3::new(1.0, 0.0, 0.0),  // 1
        Vec3::new(0.5, 1.0, 0.0),  // 2
        Vec3::new(0.5, 0.5, 1.0),  // 3 - apex of first tet
        Vec3::new(0.5, 0.5, -1.0), // 4 - apex of second tet
    ];
    // Two tets sharing face [0,1,2]
    let tets = vec![[0, 1, 2, 3], [0, 2, 1, 4]];
    let mesh = VolumeMesh::new_tet_mesh("test", vertices, tets);

    // Should have 6 exterior faces (4 per tet - 1 shared = 3 per tet * 2 = 6)
    // Not 8 faces (4 per tet * 2 = 8)
    let (_, faces) = mesh.generate_render_geometry();
    assert_eq!(faces.len(), 6, "Should only render exterior faces");
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test test_interior_face_detection -p polyscope-structures -- --nocapture`
Expected: FAIL - currently generates 8 faces instead of 6

**Step 3: Implement face counting algorithm**

Add helper function and modify `generate_render_geometry`:

```rust
use std::collections::HashMap;

/// Generates a canonical (sorted) face key for hashing.
/// For triangular faces, the fourth element is u32::MAX.
fn canonical_face_key(v0: u32, v1: u32, v2: u32, v3: Option<u32>) -> [u32; 4] {
    let mut key = [v0, v1, v2, v3.unwrap_or(u32::MAX)];
    key.sort();
    key
}

/// Face stencil for tetrahedra: 4 triangular faces
const TET_FACE_STENCIL: [[usize; 3]; 4] = [
    [0, 2, 1],
    [0, 1, 3],
    [0, 3, 2],
    [1, 2, 3],
];

/// Face stencil for hexahedra: 6 quad faces (each as 2 triangles sharing diagonal)
const HEX_FACE_STENCIL: [[[usize; 3]; 2]; 6] = [
    [[2, 1, 0], [2, 0, 3]], // Bottom
    [[4, 0, 1], [4, 1, 5]], // Front
    [[5, 1, 2], [5, 2, 6]], // Right
    [[7, 3, 0], [7, 0, 4]], // Left
    [[6, 2, 3], [6, 3, 7]], // Back
    [[7, 4, 5], [7, 5, 6]], // Top
];

impl VolumeMesh {
    fn compute_face_counts(&self) -> HashMap<[u32; 4], usize> {
        let mut face_counts: HashMap<[u32; 4], usize> = HashMap::new();

        for cell in &self.cells {
            if cell[4] == u32::MAX {
                // Tetrahedron
                for [a, b, c] in TET_FACE_STENCIL {
                    let key = canonical_face_key(cell[a], cell[b], cell[c], None);
                    *face_counts.entry(key).or_insert(0) += 1;
                }
            } else {
                // Hexahedron - each quad face uses same 4 vertices
                for quad in HEX_FACE_STENCIL {
                    // Get the 4 unique vertices of this quad face
                    let v0 = cell[quad[0][0]];
                    let v1 = cell[quad[0][1]];
                    let v2 = cell[quad[0][2]];
                    let v3 = cell[quad[1][2]]; // The fourth vertex
                    let key = canonical_face_key(v0, v1, v2, Some(v3));
                    *face_counts.entry(key).or_insert(0) += 1;
                }
            }
        }

        face_counts
    }

    /// Generates triangulated exterior faces for rendering.
    fn generate_render_geometry(&self) -> (Vec<Vec3>, Vec<[u32; 3]>) {
        let face_counts = self.compute_face_counts();
        let mut positions = Vec::new();
        let mut faces = Vec::new();

        for cell in &self.cells {
            if cell[4] == u32::MAX {
                // Tetrahedron
                for [a, b, c] in TET_FACE_STENCIL {
                    let key = canonical_face_key(cell[a], cell[b], cell[c], None);
                    if face_counts[&key] == 1 {
                        // Exterior face
                        let base_idx = positions.len() as u32;
                        positions.push(self.vertices[cell[a] as usize]);
                        positions.push(self.vertices[cell[b] as usize]);
                        positions.push(self.vertices[cell[c] as usize]);
                        faces.push([base_idx, base_idx + 1, base_idx + 2]);
                    }
                }
            } else {
                // Hexahedron
                for quad in HEX_FACE_STENCIL {
                    let v0 = cell[quad[0][0]];
                    let v1 = cell[quad[0][1]];
                    let v2 = cell[quad[0][2]];
                    let v3 = cell[quad[1][2]];
                    let key = canonical_face_key(v0, v1, v2, Some(v3));
                    if face_counts[&key] == 1 {
                        // Exterior face - emit both triangles
                        for [a, b, c] in quad {
                            let base_idx = positions.len() as u32;
                            positions.push(self.vertices[cell[a] as usize]);
                            positions.push(self.vertices[cell[b] as usize]);
                            positions.push(self.vertices[cell[c] as usize]);
                            faces.push([base_idx, base_idx + 1, base_idx + 2]);
                        }
                    }
                }
            }
        }

        (positions, faces)
    }
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test test_interior_face_detection -p polyscope-structures -- --nocapture`
Expected: PASS

**Step 5: Add more comprehensive tests**

```rust
#[test]
fn test_single_tet_all_exterior() {
    let vertices = vec![
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(1.0, 0.0, 0.0),
        Vec3::new(0.5, 1.0, 0.0),
        Vec3::new(0.5, 0.5, 1.0),
    ];
    let tets = vec![[0, 1, 2, 3]];
    let mesh = VolumeMesh::new_tet_mesh("test", vertices, tets);

    let (_, faces) = mesh.generate_render_geometry();
    assert_eq!(faces.len(), 4, "Single tet should have 4 exterior faces");
}

#[test]
fn test_single_hex_all_exterior() {
    let vertices = vec![
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(1.0, 0.0, 0.0),
        Vec3::new(1.0, 1.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
        Vec3::new(0.0, 0.0, 1.0),
        Vec3::new(1.0, 0.0, 1.0),
        Vec3::new(1.0, 1.0, 1.0),
        Vec3::new(0.0, 1.0, 1.0),
    ];
    let hexes = vec![[0, 1, 2, 3, 4, 5, 6, 7]];
    let mesh = VolumeMesh::new_hex_mesh("test", vertices, hexes);

    let (_, faces) = mesh.generate_render_geometry();
    // 6 quad faces * 2 triangles each = 12 triangles
    assert_eq!(faces.len(), 12, "Single hex should have 12 triangles (6 quads)");
}
```

**Step 6: Run all tests**

Run: `cargo test -p polyscope-structures`
Expected: All tests pass

**Step 7: Commit**

```bash
git add crates/polyscope-structures/src/volume_mesh/mod.rs
git commit -m "feat(volume_mesh): implement interior/exterior face detection

Only render exterior faces (not shared between cells) to eliminate
overdraw and properly display the mesh boundary. Uses face hashing
with sorted vertex indices to count face occurrences.

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"
```

---

## Task 2: Vertex Scalar Quantity

**Files:**
- Create: `crates/polyscope-structures/src/volume_mesh/scalar_quantity.rs`
- Modify: `crates/polyscope-structures/src/volume_mesh/mod.rs` (add module, add convenience method)
- Test: inline in scalar_quantity.rs

**Step 1: Write the failing test**

Create `crates/polyscope-structures/src/volume_mesh/scalar_quantity.rs`:

```rust
//! Scalar quantities for volume meshes.

use glam::Vec3;
use polyscope_core::quantity::{CellQuantity, Quantity, QuantityKind, VertexQuantity};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vertex_scalar_quantity_creation() {
        let values = vec![0.0, 1.0, 2.0, 3.0];
        let quantity = VolumeMeshVertexScalarQuantity::new(
            "temperature",
            "my_mesh",
            values.clone(),
        );

        assert_eq!(quantity.name(), "temperature");
        assert_eq!(quantity.values(), &values);
        assert_eq!(quantity.data_range(), (0.0, 3.0));
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test test_vertex_scalar_quantity_creation -p polyscope-structures -- --nocapture`
Expected: FAIL - struct doesn't exist

**Step 3: Implement VolumeMeshVertexScalarQuantity**

```rust
//! Scalar quantities for volume meshes.

use polyscope_core::quantity::{CellQuantity, Quantity, QuantityKind, VertexQuantity};

/// A scalar quantity defined at mesh vertices.
pub struct VolumeMeshVertexScalarQuantity {
    name: String,
    structure_name: String,
    values: Vec<f32>,
    enabled: bool,
    color_map: String,
    data_min: f32,
    data_max: f32,
}

impl VolumeMeshVertexScalarQuantity {
    /// Creates a new vertex scalar quantity.
    pub fn new(
        name: impl Into<String>,
        structure_name: impl Into<String>,
        values: Vec<f32>,
    ) -> Self {
        let (data_min, data_max) = Self::compute_range(&values);
        Self {
            name: name.into(),
            structure_name: structure_name.into(),
            values,
            enabled: false,
            color_map: "viridis".to_string(),
            data_min,
            data_max,
        }
    }

    fn compute_range(values: &[f32]) -> (f32, f32) {
        let mut min = f32::MAX;
        let mut max = f32::MIN;
        for &v in values {
            if v.is_finite() {
                min = min.min(v);
                max = max.max(v);
            }
        }
        if min > max { (0.0, 1.0) } else { (min, max) }
    }

    /// Returns the values.
    pub fn values(&self) -> &[f32] {
        &self.values
    }

    /// Gets the color map name.
    pub fn color_map(&self) -> &str {
        &self.color_map
    }

    /// Sets the color map name.
    pub fn set_color_map(&mut self, name: impl Into<String>) -> &mut Self {
        self.color_map = name.into();
        self
    }

    /// Gets the data range.
    pub fn data_range(&self) -> (f32, f32) {
        (self.data_min, self.data_max)
    }

    /// Sets the data range.
    pub fn set_data_range(&mut self, min: f32, max: f32) -> &mut Self {
        self.data_min = min;
        self.data_max = max;
        self
    }

    /// Builds egui UI for this quantity.
    pub fn build_egui_ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            let mut enabled = self.enabled;
            if ui.checkbox(&mut enabled, "").changed() {
                self.enabled = enabled;
            }
            ui.label(&self.name);
            ui.label("(vertex scalar)");
            ui.label(format!("[{:.3}, {:.3}]", self.data_min, self.data_max));
        });
    }
}

impl Quantity for VolumeMeshVertexScalarQuantity {
    fn name(&self) -> &str { &self.name }
    fn structure_name(&self) -> &str { &self.structure_name }
    fn kind(&self) -> QuantityKind { QuantityKind::Scalar }
    fn is_enabled(&self) -> bool { self.enabled }
    fn set_enabled(&mut self, enabled: bool) { self.enabled = enabled; }
    fn data_size(&self) -> usize { self.values.len() }
    fn build_ui(&mut self, _ui: &dyn std::any::Any) {}
    fn refresh(&mut self) {}
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}

impl VertexQuantity for VolumeMeshVertexScalarQuantity {}
```

**Step 4: Run test to verify it passes**

Run: `cargo test test_vertex_scalar_quantity_creation -p polyscope-structures -- --nocapture`
Expected: PASS

**Step 5: Add module to volume_mesh/mod.rs**

Add to top of `mod.rs`:
```rust
mod scalar_quantity;
pub use scalar_quantity::*;
```

**Step 6: Add convenience method to VolumeMesh**

```rust
impl VolumeMesh {
    /// Adds a vertex scalar quantity.
    pub fn add_vertex_scalar_quantity(
        &mut self,
        name: impl Into<String>,
        values: Vec<f32>,
    ) -> &mut Self {
        let name = name.into();
        let quantity = VolumeMeshVertexScalarQuantity::new(
            name.clone(),
            self.name.clone(),
            values,
        );
        self.add_quantity(Box::new(quantity));
        self
    }
}
```

**Step 7: Run all tests**

Run: `cargo test -p polyscope-structures`
Expected: All tests pass

**Step 8: Commit**

```bash
git add crates/polyscope-structures/src/volume_mesh/
git commit -m "feat(volume_mesh): add vertex scalar quantity

Adds VolumeMeshVertexScalarQuantity for visualizing scalar data
at mesh vertices. Includes color map support and data range UI.

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"
```

---

## Task 3: Cell Scalar Quantity

**Files:**
- Modify: `crates/polyscope-structures/src/volume_mesh/scalar_quantity.rs`
- Test: inline in scalar_quantity.rs

**Step 1: Write the failing test**

Add to scalar_quantity.rs tests:

```rust
#[test]
fn test_cell_scalar_quantity_creation() {
    let values = vec![0.5, 1.5];
    let quantity = VolumeMeshCellScalarQuantity::new(
        "pressure",
        "my_mesh",
        values.clone(),
    );

    assert_eq!(quantity.name(), "pressure");
    assert_eq!(quantity.values(), &values);
    assert_eq!(quantity.data_range(), (0.5, 1.5));
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test test_cell_scalar_quantity_creation -p polyscope-structures -- --nocapture`
Expected: FAIL

**Step 3: Implement VolumeMeshCellScalarQuantity**

Add to scalar_quantity.rs:

```rust
/// A scalar quantity defined at mesh cells.
pub struct VolumeMeshCellScalarQuantity {
    name: String,
    structure_name: String,
    values: Vec<f32>,
    enabled: bool,
    color_map: String,
    data_min: f32,
    data_max: f32,
}

impl VolumeMeshCellScalarQuantity {
    /// Creates a new cell scalar quantity.
    pub fn new(
        name: impl Into<String>,
        structure_name: impl Into<String>,
        values: Vec<f32>,
    ) -> Self {
        let (data_min, data_max) = Self::compute_range(&values);
        Self {
            name: name.into(),
            structure_name: structure_name.into(),
            values,
            enabled: false,
            color_map: "viridis".to_string(),
            data_min,
            data_max,
        }
    }

    fn compute_range(values: &[f32]) -> (f32, f32) {
        let mut min = f32::MAX;
        let mut max = f32::MIN;
        for &v in values {
            if v.is_finite() {
                min = min.min(v);
                max = max.max(v);
            }
        }
        if min > max { (0.0, 1.0) } else { (min, max) }
    }

    /// Returns the values.
    pub fn values(&self) -> &[f32] {
        &self.values
    }

    /// Gets the color map name.
    pub fn color_map(&self) -> &str {
        &self.color_map
    }

    /// Sets the color map name.
    pub fn set_color_map(&mut self, name: impl Into<String>) -> &mut Self {
        self.color_map = name.into();
        self
    }

    /// Gets the data range.
    pub fn data_range(&self) -> (f32, f32) {
        (self.data_min, self.data_max)
    }

    /// Sets the data range.
    pub fn set_data_range(&mut self, min: f32, max: f32) -> &mut Self {
        self.data_min = min;
        self.data_max = max;
        self
    }

    /// Builds egui UI for this quantity.
    pub fn build_egui_ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            let mut enabled = self.enabled;
            if ui.checkbox(&mut enabled, "").changed() {
                self.enabled = enabled;
            }
            ui.label(&self.name);
            ui.label("(cell scalar)");
            ui.label(format!("[{:.3}, {:.3}]", self.data_min, self.data_max));
        });
    }
}

impl Quantity for VolumeMeshCellScalarQuantity {
    fn name(&self) -> &str { &self.name }
    fn structure_name(&self) -> &str { &self.structure_name }
    fn kind(&self) -> QuantityKind { QuantityKind::Scalar }
    fn is_enabled(&self) -> bool { self.enabled }
    fn set_enabled(&mut self, enabled: bool) { self.enabled = enabled; }
    fn data_size(&self) -> usize { self.values.len() }
    fn build_ui(&mut self, _ui: &dyn std::any::Any) {}
    fn refresh(&mut self) {}
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}

impl CellQuantity for VolumeMeshCellScalarQuantity {}
```

**Step 4: Run test to verify it passes**

Run: `cargo test test_cell_scalar_quantity_creation -p polyscope-structures -- --nocapture`
Expected: PASS

**Step 5: Add convenience method to VolumeMesh**

```rust
impl VolumeMesh {
    /// Adds a cell scalar quantity.
    pub fn add_cell_scalar_quantity(
        &mut self,
        name: impl Into<String>,
        values: Vec<f32>,
    ) -> &mut Self {
        let name = name.into();
        let quantity = VolumeMeshCellScalarQuantity::new(
            name.clone(),
            self.name.clone(),
            values,
        );
        self.add_quantity(Box::new(quantity));
        self
    }
}
```

**Step 6: Run all tests**

Run: `cargo test -p polyscope-structures`
Expected: All tests pass

**Step 7: Commit**

```bash
git add crates/polyscope-structures/src/volume_mesh/
git commit -m "feat(volume_mesh): add cell scalar quantity

Adds VolumeMeshCellScalarQuantity for visualizing scalar data
at mesh cells (tets/hexes). One value per cell.

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"
```

---

## Task 4: Color Quantities (Vertex and Cell)

**Files:**
- Create: `crates/polyscope-structures/src/volume_mesh/color_quantity.rs`
- Modify: `crates/polyscope-structures/src/volume_mesh/mod.rs`
- Test: inline in color_quantity.rs

**Step 1: Write the failing test**

Create `crates/polyscope-structures/src/volume_mesh/color_quantity.rs`:

```rust
//! Color quantities for volume meshes.

use glam::Vec3;
use polyscope_core::quantity::{CellQuantity, Quantity, QuantityKind, VertexQuantity};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vertex_color_quantity() {
        let colors = vec![
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        ];
        let quantity = VolumeMeshVertexColorQuantity::new("colors", "mesh", colors.clone());

        assert_eq!(quantity.name(), "colors");
        assert_eq!(quantity.colors().len(), 2);
    }

    #[test]
    fn test_cell_color_quantity() {
        let colors = vec![Vec3::new(0.0, 0.0, 1.0)];
        let quantity = VolumeMeshCellColorQuantity::new("cell_colors", "mesh", colors.clone());

        assert_eq!(quantity.name(), "cell_colors");
        assert_eq!(quantity.colors().len(), 1);
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p polyscope-structures color_quantity -- --nocapture`
Expected: FAIL

**Step 3: Implement color quantities**

```rust
//! Color quantities for volume meshes.

use glam::Vec3;
use polyscope_core::quantity::{CellQuantity, Quantity, QuantityKind, VertexQuantity};

/// A color quantity defined at mesh vertices.
pub struct VolumeMeshVertexColorQuantity {
    name: String,
    structure_name: String,
    colors: Vec<Vec3>,
    enabled: bool,
}

impl VolumeMeshVertexColorQuantity {
    pub fn new(
        name: impl Into<String>,
        structure_name: impl Into<String>,
        colors: Vec<Vec3>,
    ) -> Self {
        Self {
            name: name.into(),
            structure_name: structure_name.into(),
            colors,
            enabled: false,
        }
    }

    pub fn colors(&self) -> &[Vec3] {
        &self.colors
    }

    pub fn build_egui_ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            let mut enabled = self.enabled;
            if ui.checkbox(&mut enabled, "").changed() {
                self.enabled = enabled;
            }
            ui.label(&self.name);
            ui.label("(vertex color)");
        });
    }
}

impl Quantity for VolumeMeshVertexColorQuantity {
    fn name(&self) -> &str { &self.name }
    fn structure_name(&self) -> &str { &self.structure_name }
    fn kind(&self) -> QuantityKind { QuantityKind::Color }
    fn is_enabled(&self) -> bool { self.enabled }
    fn set_enabled(&mut self, enabled: bool) { self.enabled = enabled; }
    fn data_size(&self) -> usize { self.colors.len() }
    fn build_ui(&mut self, _ui: &dyn std::any::Any) {}
    fn refresh(&mut self) {}
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}

impl VertexQuantity for VolumeMeshVertexColorQuantity {}

/// A color quantity defined at mesh cells.
pub struct VolumeMeshCellColorQuantity {
    name: String,
    structure_name: String,
    colors: Vec<Vec3>,
    enabled: bool,
}

impl VolumeMeshCellColorQuantity {
    pub fn new(
        name: impl Into<String>,
        structure_name: impl Into<String>,
        colors: Vec<Vec3>,
    ) -> Self {
        Self {
            name: name.into(),
            structure_name: structure_name.into(),
            colors,
            enabled: false,
        }
    }

    pub fn colors(&self) -> &[Vec3] {
        &self.colors
    }

    pub fn build_egui_ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            let mut enabled = self.enabled;
            if ui.checkbox(&mut enabled, "").changed() {
                self.enabled = enabled;
            }
            ui.label(&self.name);
            ui.label("(cell color)");
        });
    }
}

impl Quantity for VolumeMeshCellColorQuantity {
    fn name(&self) -> &str { &self.name }
    fn structure_name(&self) -> &str { &self.structure_name }
    fn kind(&self) -> QuantityKind { QuantityKind::Color }
    fn is_enabled(&self) -> bool { self.enabled }
    fn set_enabled(&mut self, enabled: bool) { self.enabled = enabled; }
    fn data_size(&self) -> usize { self.colors.len() }
    fn build_ui(&mut self, _ui: &dyn std::any::Any) {}
    fn refresh(&mut self) {}
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}

impl CellQuantity for VolumeMeshCellColorQuantity {}
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p polyscope-structures color_quantity -- --nocapture`
Expected: PASS

**Step 5: Add module and convenience methods**

In mod.rs:
```rust
mod color_quantity;
pub use color_quantity::*;
```

Add to VolumeMesh impl:
```rust
/// Adds a vertex color quantity.
pub fn add_vertex_color_quantity(
    &mut self,
    name: impl Into<String>,
    colors: Vec<Vec3>,
) -> &mut Self {
    let name = name.into();
    let quantity = VolumeMeshVertexColorQuantity::new(
        name.clone(),
        self.name.clone(),
        colors,
    );
    self.add_quantity(Box::new(quantity));
    self
}

/// Adds a cell color quantity.
pub fn add_cell_color_quantity(
    &mut self,
    name: impl Into<String>,
    colors: Vec<Vec3>,
) -> &mut Self {
    let name = name.into();
    let quantity = VolumeMeshCellColorQuantity::new(
        name.clone(),
        self.name.clone(),
        colors,
    );
    self.add_quantity(Box::new(quantity));
    self
}
```

**Step 6: Run all tests**

Run: `cargo test -p polyscope-structures`
Expected: All tests pass

**Step 7: Commit**

```bash
git add crates/polyscope-structures/src/volume_mesh/
git commit -m "feat(volume_mesh): add color quantities

Adds VolumeMeshVertexColorQuantity and VolumeMeshCellColorQuantity
for RGB color visualization at vertices and cells.

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"
```

---

## Task 5: Vector Quantities (Vertex and Cell)

**Files:**
- Create: `crates/polyscope-structures/src/volume_mesh/vector_quantity.rs`
- Modify: `crates/polyscope-structures/src/volume_mesh/mod.rs`
- Test: inline in vector_quantity.rs

**Step 1: Write the failing test**

Create `crates/polyscope-structures/src/volume_mesh/vector_quantity.rs`:

```rust
//! Vector quantities for volume meshes.

use glam::Vec3;
use polyscope_core::quantity::{CellQuantity, Quantity, QuantityKind, VertexQuantity};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vertex_vector_quantity() {
        let vectors = vec![
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        ];
        let quantity = VolumeMeshVertexVectorQuantity::new("velocity", "mesh", vectors.clone());

        assert_eq!(quantity.name(), "velocity");
        assert_eq!(quantity.vectors().len(), 2);
    }

    #[test]
    fn test_cell_vector_quantity() {
        let vectors = vec![Vec3::new(0.0, 0.0, 1.0)];
        let quantity = VolumeMeshCellVectorQuantity::new("flux", "mesh", vectors.clone());

        assert_eq!(quantity.name(), "flux");
        assert_eq!(quantity.vectors().len(), 1);
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p polyscope-structures vector_quantity -- --nocapture`
Expected: FAIL

**Step 3: Implement vector quantities**

```rust
//! Vector quantities for volume meshes.

use glam::Vec3;
use polyscope_core::quantity::{CellQuantity, Quantity, QuantityKind, VertexQuantity};

/// Vector field visualization style.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VectorStyle {
    #[default]
    Arrow,
    Line,
}

/// A vector quantity defined at mesh vertices.
pub struct VolumeMeshVertexVectorQuantity {
    name: String,
    structure_name: String,
    vectors: Vec<Vec3>,
    enabled: bool,
    vector_length_scale: f32,
    vector_radius: f32,
    vector_color: Vec3,
    style: VectorStyle,
}

impl VolumeMeshVertexVectorQuantity {
    pub fn new(
        name: impl Into<String>,
        structure_name: impl Into<String>,
        vectors: Vec<Vec3>,
    ) -> Self {
        Self {
            name: name.into(),
            structure_name: structure_name.into(),
            vectors,
            enabled: false,
            vector_length_scale: 1.0,
            vector_radius: 0.01,
            vector_color: Vec3::new(0.1, 0.1, 0.8),
            style: VectorStyle::Arrow,
        }
    }

    pub fn vectors(&self) -> &[Vec3] {
        &self.vectors
    }

    pub fn set_length_scale(&mut self, scale: f32) -> &mut Self {
        self.vector_length_scale = scale;
        self
    }

    pub fn set_radius(&mut self, radius: f32) -> &mut Self {
        self.vector_radius = radius;
        self
    }

    pub fn set_color(&mut self, color: Vec3) -> &mut Self {
        self.vector_color = color;
        self
    }

    pub fn build_egui_ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            let mut enabled = self.enabled;
            if ui.checkbox(&mut enabled, "").changed() {
                self.enabled = enabled;
            }
            ui.label(&self.name);
            ui.label("(vertex vector)");
        });

        if self.enabled {
            ui.horizontal(|ui| {
                ui.label("Length:");
                ui.add(egui::DragValue::new(&mut self.vector_length_scale)
                    .speed(0.01)
                    .range(0.001..=10.0));
            });
        }
    }
}

impl Quantity for VolumeMeshVertexVectorQuantity {
    fn name(&self) -> &str { &self.name }
    fn structure_name(&self) -> &str { &self.structure_name }
    fn kind(&self) -> QuantityKind { QuantityKind::Vector }
    fn is_enabled(&self) -> bool { self.enabled }
    fn set_enabled(&mut self, enabled: bool) { self.enabled = enabled; }
    fn data_size(&self) -> usize { self.vectors.len() }
    fn build_ui(&mut self, _ui: &dyn std::any::Any) {}
    fn refresh(&mut self) {}
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}

impl VertexQuantity for VolumeMeshVertexVectorQuantity {}

/// A vector quantity defined at mesh cells.
pub struct VolumeMeshCellVectorQuantity {
    name: String,
    structure_name: String,
    vectors: Vec<Vec3>,
    enabled: bool,
    vector_length_scale: f32,
    vector_radius: f32,
    vector_color: Vec3,
    style: VectorStyle,
}

impl VolumeMeshCellVectorQuantity {
    pub fn new(
        name: impl Into<String>,
        structure_name: impl Into<String>,
        vectors: Vec<Vec3>,
    ) -> Self {
        Self {
            name: name.into(),
            structure_name: structure_name.into(),
            vectors,
            enabled: false,
            vector_length_scale: 1.0,
            vector_radius: 0.01,
            vector_color: Vec3::new(0.1, 0.1, 0.8),
            style: VectorStyle::Arrow,
        }
    }

    pub fn vectors(&self) -> &[Vec3] {
        &self.vectors
    }

    pub fn set_length_scale(&mut self, scale: f32) -> &mut Self {
        self.vector_length_scale = scale;
        self
    }

    pub fn set_radius(&mut self, radius: f32) -> &mut Self {
        self.vector_radius = radius;
        self
    }

    pub fn set_color(&mut self, color: Vec3) -> &mut Self {
        self.vector_color = color;
        self
    }

    pub fn build_egui_ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            let mut enabled = self.enabled;
            if ui.checkbox(&mut enabled, "").changed() {
                self.enabled = enabled;
            }
            ui.label(&self.name);
            ui.label("(cell vector)");
        });

        if self.enabled {
            ui.horizontal(|ui| {
                ui.label("Length:");
                ui.add(egui::DragValue::new(&mut self.vector_length_scale)
                    .speed(0.01)
                    .range(0.001..=10.0));
            });
        }
    }
}

impl Quantity for VolumeMeshCellVectorQuantity {
    fn name(&self) -> &str { &self.name }
    fn structure_name(&self) -> &str { &self.structure_name }
    fn kind(&self) -> QuantityKind { QuantityKind::Vector }
    fn is_enabled(&self) -> bool { self.enabled }
    fn set_enabled(&mut self, enabled: bool) { self.enabled = enabled; }
    fn data_size(&self) -> usize { self.vectors.len() }
    fn build_ui(&mut self, _ui: &dyn std::any::Any) {}
    fn refresh(&mut self) {}
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}

impl CellQuantity for VolumeMeshCellVectorQuantity {}
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p polyscope-structures vector_quantity -- --nocapture`
Expected: PASS

**Step 5: Add module and convenience methods**

In mod.rs:
```rust
mod vector_quantity;
pub use vector_quantity::*;
```

Add to VolumeMesh impl:
```rust
/// Adds a vertex vector quantity.
pub fn add_vertex_vector_quantity(
    &mut self,
    name: impl Into<String>,
    vectors: Vec<Vec3>,
) -> &mut Self {
    let name = name.into();
    let quantity = VolumeMeshVertexVectorQuantity::new(
        name.clone(),
        self.name.clone(),
        vectors,
    );
    self.add_quantity(Box::new(quantity));
    self
}

/// Adds a cell vector quantity.
pub fn add_cell_vector_quantity(
    &mut self,
    name: impl Into<String>,
    vectors: Vec<Vec3>,
) -> &mut Self {
    let name = name.into();
    let quantity = VolumeMeshCellVectorQuantity::new(
        name.clone(),
        self.name.clone(),
        vectors,
    );
    self.add_quantity(Box::new(quantity));
    self
}
```

**Step 6: Run all tests**

Run: `cargo test -p polyscope-structures`
Expected: All tests pass

**Step 7: Commit**

```bash
git add crates/polyscope-structures/src/volume_mesh/
git commit -m "feat(volume_mesh): add vector quantities

Adds VolumeMeshVertexVectorQuantity and VolumeMeshCellVectorQuantity
for vector field visualization with configurable length, radius, color.

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"
```

---

## Task 6: Quantity-Aware Rendering

**Files:**
- Modify: `crates/polyscope-structures/src/volume_mesh/mod.rs`
- Modify: `crates/polyscope-render/src/shaders/` (if needed for color-mapped rendering)

This task connects quantities to the rendering pipeline so enabled quantities actually affect visualization.

**Step 1: Write failing test**

```rust
#[test]
fn test_quantity_aware_geometry_generation() {
    let vertices = vec![
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(1.0, 0.0, 0.0),
        Vec3::new(0.5, 1.0, 0.0),
        Vec3::new(0.5, 0.5, 1.0),
    ];
    let tets = vec![[0, 1, 2, 3]];
    let mut mesh = VolumeMesh::new_tet_mesh("test", vertices, tets);

    // Add vertex scalar quantity
    mesh.add_vertex_scalar_quantity("temp", vec![0.0, 0.5, 1.0, 0.25]);

    // Get the quantity and enable it
    if let Some(q) = mesh.get_quantity_mut("temp") {
        q.set_enabled(true);
    }

    // Generate geometry should include scalar values for color mapping
    let render_data = mesh.generate_render_geometry_with_quantities();
    assert!(render_data.vertex_values.is_some());
    assert_eq!(render_data.vertex_values.as_ref().unwrap().len(),
               render_data.positions.len());
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test test_quantity_aware_geometry_generation -p polyscope-structures -- --nocapture`
Expected: FAIL

**Step 3: Implement quantity-aware geometry generation**

Add new struct and method:

```rust
/// Render geometry data with optional quantity values.
pub struct VolumeMeshRenderGeometry {
    pub positions: Vec<Vec3>,
    pub faces: Vec<[u32; 3]>,
    pub normals: Vec<Vec3>,
    /// Per-vertex scalar values for color mapping (from enabled vertex scalar quantity).
    pub vertex_values: Option<Vec<f32>>,
    /// Per-vertex colors (from enabled vertex color quantity).
    pub vertex_colors: Option<Vec<Vec3>>,
}

impl VolumeMesh {
    /// Generates render geometry including any enabled quantity data.
    pub fn generate_render_geometry_with_quantities(&self) -> VolumeMeshRenderGeometry {
        let face_counts = self.compute_face_counts();
        let mut positions = Vec::new();
        let mut faces = Vec::new();
        let mut vertex_indices = Vec::new(); // Track original vertex indices
        let mut cell_indices = Vec::new();   // Track which cell each face belongs to

        // First pass: generate geometry and track indices
        for (cell_idx, cell) in self.cells.iter().enumerate() {
            if cell[4] == u32::MAX {
                // Tetrahedron
                for [a, b, c] in TET_FACE_STENCIL {
                    let key = canonical_face_key(cell[a], cell[b], cell[c], None);
                    if face_counts[&key] == 1 {
                        let base_idx = positions.len() as u32;
                        positions.push(self.vertices[cell[a] as usize]);
                        positions.push(self.vertices[cell[b] as usize]);
                        positions.push(self.vertices[cell[c] as usize]);
                        vertex_indices.push(cell[a] as usize);
                        vertex_indices.push(cell[b] as usize);
                        vertex_indices.push(cell[c] as usize);
                        cell_indices.push(cell_idx);
                        cell_indices.push(cell_idx);
                        cell_indices.push(cell_idx);
                        faces.push([base_idx, base_idx + 1, base_idx + 2]);
                    }
                }
            } else {
                // Hexahedron
                for quad in HEX_FACE_STENCIL {
                    let v0 = cell[quad[0][0]];
                    let v1 = cell[quad[0][1]];
                    let v2 = cell[quad[0][2]];
                    let v3 = cell[quad[1][2]];
                    let key = canonical_face_key(v0, v1, v2, Some(v3));
                    if face_counts[&key] == 1 {
                        for [a, b, c] in quad {
                            let base_idx = positions.len() as u32;
                            positions.push(self.vertices[cell[a] as usize]);
                            positions.push(self.vertices[cell[b] as usize]);
                            positions.push(self.vertices[cell[c] as usize]);
                            vertex_indices.push(cell[a] as usize);
                            vertex_indices.push(cell[b] as usize);
                            vertex_indices.push(cell[c] as usize);
                            cell_indices.push(cell_idx);
                            cell_indices.push(cell_idx);
                            cell_indices.push(cell_idx);
                            faces.push([base_idx, base_idx + 1, base_idx + 2]);
                        }
                    }
                }
            }
        }

        // Compute normals
        let mut normals = vec![Vec3::ZERO; positions.len()];
        for [a, b, c] in &faces {
            let p0 = positions[*a as usize];
            let p1 = positions[*b as usize];
            let p2 = positions[*c as usize];
            let normal = (p1 - p0).cross(p2 - p0).normalize_or_zero();
            normals[*a as usize] = normal;
            normals[*b as usize] = normal;
            normals[*c as usize] = normal;
        }

        // Find enabled scalar quantity
        let mut vertex_values = None;
        let mut vertex_colors = None;

        for q in &self.quantities {
            if q.is_enabled() {
                if let Some(scalar) = q.as_any().downcast_ref::<VolumeMeshVertexScalarQuantity>() {
                    let values: Vec<f32> = vertex_indices.iter()
                        .map(|&idx| scalar.values().get(idx).copied().unwrap_or(0.0))
                        .collect();
                    vertex_values = Some(values);
                    break;
                }
                if let Some(color) = q.as_any().downcast_ref::<VolumeMeshVertexColorQuantity>() {
                    let colors: Vec<Vec3> = vertex_indices.iter()
                        .map(|&idx| color.colors().get(idx).copied().unwrap_or(Vec3::ONE))
                        .collect();
                    vertex_colors = Some(colors);
                    break;
                }
                if let Some(scalar) = q.as_any().downcast_ref::<VolumeMeshCellScalarQuantity>() {
                    let values: Vec<f32> = cell_indices.iter()
                        .map(|&idx| scalar.values().get(idx).copied().unwrap_or(0.0))
                        .collect();
                    vertex_values = Some(values);
                    break;
                }
                if let Some(color) = q.as_any().downcast_ref::<VolumeMeshCellColorQuantity>() {
                    let colors: Vec<Vec3> = cell_indices.iter()
                        .map(|&idx| color.colors().get(idx).copied().unwrap_or(Vec3::ONE))
                        .collect();
                    vertex_colors = Some(colors);
                    break;
                }
            }
        }

        VolumeMeshRenderGeometry {
            positions,
            faces,
            normals,
            vertex_values,
            vertex_colors,
        }
    }
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test test_quantity_aware_geometry_generation -p polyscope-structures -- --nocapture`
Expected: PASS

**Step 5: Update init_render_data to use new method**

Modify `init_render_data` to use `generate_render_geometry_with_quantities` and pass scalar/color data to GPU buffers.

**Step 6: Run all tests**

Run: `cargo test -p polyscope-structures`
Expected: All tests pass

**Step 7: Commit**

```bash
git add crates/polyscope-structures/src/volume_mesh/
git commit -m "feat(volume_mesh): connect quantities to rendering pipeline

Quantities now affect rendering when enabled. Vertex/cell scalar
quantities use color mapping, color quantities use direct colors.

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"
```

---

## Task 7: Hex to Tet Decomposition for Slice Planes

**Files:**
- Modify: `crates/polyscope-structures/src/volume_mesh/mod.rs`
- Test: inline tests

This task adds tet decomposition for hexahedra, needed for slice plane rendering (C++ uses SLICE_TETS shader which expects tets).

**Step 1: Write failing test**

```rust
#[test]
fn test_hex_to_tet_decomposition() {
    let vertices = vec![
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(1.0, 0.0, 0.0),
        Vec3::new(1.0, 1.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
        Vec3::new(0.0, 0.0, 1.0),
        Vec3::new(1.0, 0.0, 1.0),
        Vec3::new(1.0, 1.0, 1.0),
        Vec3::new(0.0, 1.0, 1.0),
    ];
    let hexes = vec![[0, 1, 2, 3, 4, 5, 6, 7]];
    let mesh = VolumeMesh::new_hex_mesh("test", vertices, hexes);

    let tets = mesh.decompose_to_tets();
    // A hex can be decomposed into 5 or 6 tets
    assert!(tets.len() >= 5 && tets.len() <= 6);

    // Each tet should have 4 vertices
    for tet in &tets {
        assert!(tet[0] < 8);
        assert!(tet[1] < 8);
        assert!(tet[2] < 8);
        assert!(tet[3] < 8);
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test test_hex_to_tet_decomposition -p polyscope-structures -- --nocapture`
Expected: FAIL

**Step 3: Implement hex decomposition**

Based on C++ Polyscope's `rotationMap` and `diagonalMap`:

```rust
/// Rotation map for hex vertices to place vertex 0 in canonical position.
const HEX_ROTATION_MAP: [[usize; 8]; 8] = [
    [0, 1, 2, 3, 4, 5, 7, 6],
    [1, 0, 4, 5, 2, 3, 6, 7],
    [2, 1, 5, 6, 3, 0, 7, 4],
    [3, 0, 1, 2, 7, 4, 6, 5],
    [4, 0, 3, 7, 5, 1, 6, 2],
    [5, 1, 0, 4, 7, 2, 6, 3],
    [7, 3, 2, 6, 4, 0, 5, 1],
    [6, 2, 1, 5, 7, 3, 4, 0],
];

/// Diagonal decomposition patterns (5 or 6 tets depending on diagonal choice).
const HEX_DIAGONAL_MAP: [[[usize; 4]; 6]; 4] = [
    [[0, 1, 2, 5], [0, 2, 6, 5], [0, 2, 3, 6], [0, 5, 6, 4], [2, 6, 5, 7], [0, 0, 0, 0]],
    [[0, 5, 6, 4], [0, 1, 6, 5], [1, 7, 6, 5], [0, 6, 2, 3], [0, 6, 1, 2], [1, 6, 7, 2]],
    [[0, 4, 5, 7], [0, 3, 6, 7], [0, 6, 4, 7], [0, 1, 2, 5], [0, 3, 7, 2], [0, 7, 5, 2]],
    [[0, 2, 3, 7], [0, 3, 6, 7], [0, 6, 4, 7], [0, 5, 7, 4], [1, 5, 7, 0], [1, 7, 2, 0]],
];

impl VolumeMesh {
    /// Decomposes all cells into tetrahedra.
    /// Tets pass through unchanged, hexes are decomposed into 5-6 tets.
    pub fn decompose_to_tets(&self) -> Vec<[u32; 4]> {
        let mut tets = Vec::new();

        for cell in &self.cells {
            if cell[4] == u32::MAX {
                // Already a tet
                tets.push([cell[0], cell[1], cell[2], cell[3]]);
            } else {
                // Hex - decompose using diagonal pattern
                // Choose diagonal based on smallest vertex index position
                let min_idx = (0..8).min_by_key(|&i| cell[i]).unwrap();
                let rotation = &HEX_ROTATION_MAP[min_idx];

                // Use diagonal pattern 0 (5 tets)
                for tet_local in HEX_DIAGONAL_MAP[0].iter() {
                    if tet_local[0] == 0 && tet_local[1] == 0 && tet_local[2] == 0 {
                        break; // Sentinel for end of pattern
                    }
                    let tet = [
                        cell[rotation[tet_local[0]]],
                        cell[rotation[tet_local[1]]],
                        cell[rotation[tet_local[2]]],
                        cell[rotation[tet_local[3]]],
                    ];
                    tets.push(tet);
                }
            }
        }

        tets
    }

    /// Returns the number of tetrahedra (including decomposed hexes).
    pub fn num_tets(&self) -> usize {
        self.decompose_to_tets().len()
    }
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test test_hex_to_tet_decomposition -p polyscope-structures -- --nocapture`
Expected: PASS

**Step 5: Run all tests**

Run: `cargo test -p polyscope-structures`
Expected: All tests pass

**Step 6: Commit**

```bash
git add crates/polyscope-structures/src/volume_mesh/
git commit -m "feat(volume_mesh): add hex to tet decomposition

Decomposes hexahedra into 5-6 tetrahedra using diagonal patterns
from C++ Polyscope. Required for slice plane rendering.

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"
```

---

## Task 8: Integration Test with Full Rendering

**Files:**
- Create: `tests/volume_mesh_test.rs` or add to existing integration tests

**Step 1: Write integration test**

```rust
#[test]
fn test_volume_mesh_full_rendering() {
    // Create a small tet mesh
    let vertices = vec![
        glam::Vec3::new(0.0, 0.0, 0.0),
        glam::Vec3::new(1.0, 0.0, 0.0),
        glam::Vec3::new(0.5, 1.0, 0.0),
        glam::Vec3::new(0.5, 0.5, 1.0),
        glam::Vec3::new(0.5, 0.5, -1.0),
    ];
    let tets = vec![[0, 1, 2, 3], [0, 2, 1, 4]];

    let mut mesh = polyscope_structures::VolumeMesh::new_tet_mesh("test_vol", vertices, tets);

    // Add quantities
    mesh.add_vertex_scalar_quantity("temperature", vec![0.0, 0.25, 0.5, 0.75, 1.0]);
    mesh.add_cell_scalar_quantity("pressure", vec![1.0, 2.0]);

    // Verify structure
    assert_eq!(mesh.num_vertices(), 5);
    assert_eq!(mesh.num_cells(), 2);

    // Verify render geometry has correct face count (6 exterior faces)
    let geom = mesh.generate_render_geometry_with_quantities();
    assert_eq!(geom.faces.len(), 6);
}
```

**Step 2: Run test**

Run: `cargo test test_volume_mesh_full_rendering`
Expected: PASS

**Step 3: Commit**

```bash
git add tests/
git commit -m "test(volume_mesh): add integration test for full rendering

Tests mesh creation, quantity attachment, and exterior face detection.

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"
```

---

## Task 9: Documentation and Examples

**Files:**
- Update: `crates/polyscope-structures/src/volume_mesh/mod.rs` (doc comments)
- Create: `examples/volume_mesh.rs` (optional, if examples exist)

**Step 1: Add comprehensive doc comments**

Add module-level documentation:

```rust
//! Volume mesh structure for tetrahedral and hexahedral meshes.
//!
//! # Overview
//!
//! `VolumeMesh` supports both tetrahedral (4 vertices) and hexahedral (8 vertices)
//! cells. Mixed meshes are supported by using 8-index cells where unused indices
//! are set to `u32::MAX`.
//!
//! # Interior/Exterior Faces
//!
//! Only exterior faces (not shared between cells) are rendered. This is determined
//! by hashing sorted face vertex indices and counting occurrences.
//!
//! # Quantities
//!
//! Supported quantities:
//! - `VolumeMeshVertexScalarQuantity` - scalar per vertex
//! - `VolumeMeshCellScalarQuantity` - scalar per cell
//! - `VolumeMeshVertexColorQuantity` - RGB color per vertex
//! - `VolumeMeshCellColorQuantity` - RGB color per cell
//! - `VolumeMeshVertexVectorQuantity` - vector per vertex
//! - `VolumeMeshCellVectorQuantity` - vector per cell
//!
//! # Example
//!
//! ```rust
//! use glam::Vec3;
//! use polyscope_structures::VolumeMesh;
//!
//! // Create a single tetrahedron
//! let vertices = vec![
//!     Vec3::new(0.0, 0.0, 0.0),
//!     Vec3::new(1.0, 0.0, 0.0),
//!     Vec3::new(0.5, 1.0, 0.0),
//!     Vec3::new(0.5, 0.5, 1.0),
//! ];
//! let tets = vec![[0, 1, 2, 3]];
//! let mut mesh = VolumeMesh::new_tet_mesh("my_tet", vertices, tets);
//!
//! // Add a scalar quantity
//! mesh.add_vertex_scalar_quantity("temperature", vec![0.0, 0.5, 1.0, 0.25]);
//! ```
```

**Step 2: Verify docs build**

Run: `cargo doc -p polyscope-structures --no-deps`
Expected: No warnings, docs build successfully

**Step 3: Commit**

```bash
git add crates/polyscope-structures/src/volume_mesh/
git commit -m "docs(volume_mesh): add comprehensive documentation

Adds module-level docs explaining interior/exterior faces,
supported quantities, and usage examples.

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"
```

---

## Summary

This plan completes the volume mesh implementation with:

1. **Interior/exterior face detection** - Only render boundary faces
2. **Vertex scalar quantities** - Color-mapped scalar data at vertices
3. **Cell scalar quantities** - Color-mapped scalar data at cells
4. **Color quantities** - Direct RGB colors at vertices/cells
5. **Vector quantities** - Vector field visualization
6. **Quantity-aware rendering** - Connect quantities to GPU pipeline
7. **Hex decomposition** - Support for slice plane rendering
8. **Integration tests** - Verify full functionality
9. **Documentation** - Comprehensive docs and examples

**Slice plane integration** is deferred to the separate "Slice Plane Capping" task which requires the slice plane infrastructure to be implemented first.
