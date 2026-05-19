# Volume Mesh Prism & Pyramid Cell Support Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add prism (6-vertex wedge) and pyramid (5-vertex) cell support to `VolumeMesh`, achieving feature parity with upstream C++ Polyscope PR #353.

**Architecture:** Extract cell-shape data (stencils, polygon faces, decomposition patterns) into a new `cell_data.rs` submodule under `polyscope-structures/src/volume_mesh/`. Switch `VolumeMesh::cell_type()` from the current sentinel-slot-4 check to a sentinel-count classifier (0=Hex, 2=Prism, 3=Pyramid, 4=Tet — matches upstream). Refactor each per-cell iteration site (`compute_face_counts`, `generate_render_geometry`, `generate_render_geometry_with_culling`, `generate_render_geometry_with_quantities`, `generate_cell_index_per_triangle`, `decompose_to_tets`, `cell_centroid`) to dispatch through a unified `face_data_for(cell_type)` table instead of branching on tet-vs-hex. Add `slice_prism` / `slice_pyramid` to `slice_geometry.rs` using tet-decomposition (same strategy as existing `slice_hex`). Expose `new_prism_mesh` / `new_pyramid_mesh` on `VolumeMesh` and matching `register_*` functions on the public crate.

**Tech Stack:** Rust, glam (math), wgpu/egui (rendering — unchanged), cargo test, cargo clippy.

**File-size note:** `volume_mesh/mod.rs` is currently 1569 lines. Adding four-way matches everywhere without refactoring would push it past the 2000-line project limit. The plan extracts ~150 lines into a new `cell_data.rs` and replaces if/else branches with helper dispatch — net growth in `mod.rs` should be small.

---

## File Structure

- **New:** `crates/polyscope-structures/src/volume_mesh/cell_data.rs` — static tables for each cell type (face polygons, triangulation stencils, tet-decomposition patterns), plus `canonical_face_key` and dispatch helpers `face_data_for`, `face_polygon_for`, `decompose_cell_to_tets`. Single source of truth for shape-data.
- **Modify:** `crates/polyscope-structures/src/volume_mesh/mod.rs` — extend `VolumeCellType` enum, switch `cell_type()` to sentinel-count, replace per-cell tet-vs-hex branches with calls into `cell_data` helpers. Delete the now-relocated constants (`TET_FACE_STENCIL`, `HEX_FACE_STENCIL`, `HEX_TO_TET_PATTERN`, `canonical_face_key`).
- **Modify:** `crates/polyscope-structures/src/volume_mesh/slice_geometry.rs` — add `slice_prism` and `slice_pyramid` alongside `slice_tet`/`slice_hex`.
- **Modify:** `crates/polyscope/src/volume_mesh.rs` — add `register_prism_mesh` and `register_pyramid_mesh` next to the existing `register_tet_mesh`/`register_hex_mesh`.
- **Modify:** `crates/polyscope/src/lib.rs` — re-export the two new register functions if needed (check existing export pattern in the same file).
- **New:** `examples/volume_mesh_mixed_cells_demo.rs` — visual demo registering one mesh of each cell type plus one mixed mesh.
- **Modify:** `docs/feature-status.md` — mark prism/pyramid support as complete under "Completed Features".
- **Modify:** `CHANGELOG.md` — add entry under a new `## [Unreleased]` (or next version) section.

---

### Task 1: Create `cell_data` submodule with new enum variants

**Files:**
- Create: `crates/polyscope-structures/src/volume_mesh/cell_data.rs`
- Modify: `crates/polyscope-structures/src/volume_mesh/mod.rs` (declare submodule, extend enum)

- [ ] **Step 1: Write the failing test**

Append to the `#[cfg(test)] mod tests` block at the bottom of `crates/polyscope-structures/src/volume_mesh/mod.rs`:

```rust
#[test]
fn test_cell_type_enum_has_prism_and_pyramid() {
    // Compile-time check: pattern match must be exhaustive over all 4 variants.
    let types = [
        VolumeCellType::Tet,
        VolumeCellType::Hex,
        VolumeCellType::Prism,
        VolumeCellType::Pyramid,
    ];
    for t in types {
        let label = match t {
            VolumeCellType::Tet => "tet",
            VolumeCellType::Hex => "hex",
            VolumeCellType::Prism => "prism",
            VolumeCellType::Pyramid => "pyramid",
        };
        assert!(!label.is_empty());
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p polyscope-structures test_cell_type_enum_has_prism_and_pyramid -- --nocapture`
Expected: FAIL with "no variant or associated item named `Prism`".

- [ ] **Step 3: Extend the enum**

In `crates/polyscope-structures/src/volume_mesh/mod.rs`, replace lines 64-71 (the `VolumeCellType` enum) with:

```rust
/// Cell type for volume meshes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VolumeCellType {
    /// Tetrahedron (4 vertices, 4 triangular faces)
    Tet,
    /// Hexahedron (8 vertices, 6 quadrilateral faces)
    Hex,
    /// Triangular prism / wedge (6 vertices, 2 tri + 3 quad faces)
    Prism,
    /// Square pyramid (5 vertices, 1 quad + 4 tri faces)
    Pyramid,
}
```

- [ ] **Step 4: Create the new submodule**

Create `crates/polyscope-structures/src/volume_mesh/cell_data.rs` with the following content:

```rust
//! Static shape data for each `VolumeCellType`.
//!
//! Defines, for each cell type:
//! - the unique vertex indices on each face (used for canonical face hashing)
//! - the per-face triangulation stencil (used for rendering and edge detection)
//! - the tet-decomposition pattern (used for slicing and isosurface extraction)
//!
//! Index conventions mirror upstream C++ Polyscope (`src/volume_mesh.cpp`,
//! `stencilTet`, `stencilHex`, `stencilPrism`, `stencilPyramid`):
//! - Tet: 4 verts in slots 0..3, sentinels in 4..7. 4 triangular faces.
//! - Hex: 8 verts in 0..7. 6 quadrilateral faces, each split into 2 triangles.
//! - Prism: 6 verts in 0..5, sentinels in 6..7. Bottom tri (0,1,2), top tri (3,4,5).
//!   5 faces: 1 tri (bottom) + 3 quads (sides) + 1 tri (top) = 8 triangles.
//! - Pyramid: 5 verts in 0..4, sentinels in 5..7. Base quad (0,1,2,3), apex (4).
//!   5 faces: 1 quad (base) + 4 tris (sides) = 6 triangles.

use super::VolumeCellType;

/// A face is described by (unique vertex slot list, triangulation).
///
/// `polygon` lists the unique cell-local vertex slots in CCW order around the
/// face (3 for triangles, 4 for quads). `triangulation` is the list of triangle
/// stencils used to render the face.
pub struct FaceData {
    pub polygon: &'static [usize],
    pub triangulation: &'static [[usize; 3]],
}

// ===== Tet =====
const TET_FACES: &[FaceData] = &[
    FaceData { polygon: &[0, 2, 1], triangulation: &[[0, 2, 1]] },
    FaceData { polygon: &[0, 1, 3], triangulation: &[[0, 1, 3]] },
    FaceData { polygon: &[0, 3, 2], triangulation: &[[0, 3, 2]] },
    FaceData { polygon: &[1, 2, 3], triangulation: &[[1, 2, 3]] },
];

// ===== Hex =====
// Numbered like in the VTK file-formats diagram, with slots 6 and 7 swapped
// to match upstream (see polyscope/src/volume_mesh.cpp:43).
const HEX_FACES: &[FaceData] = &[
    FaceData { polygon: &[2, 1, 0, 3], triangulation: &[[2, 1, 0], [2, 0, 3]] }, // Bottom
    FaceData { polygon: &[4, 0, 1, 5], triangulation: &[[4, 0, 1], [4, 1, 5]] }, // Front
    FaceData { polygon: &[5, 1, 2, 6], triangulation: &[[5, 1, 2], [5, 2, 6]] }, // Right
    FaceData { polygon: &[7, 3, 0, 4], triangulation: &[[7, 3, 0], [7, 0, 4]] }, // Left
    FaceData { polygon: &[6, 2, 3, 7], triangulation: &[[6, 2, 3], [6, 3, 7]] }, // Back
    FaceData { polygon: &[7, 4, 5, 6], triangulation: &[[7, 4, 5], [7, 5, 6]] }, // Top
];

// ===== Prism (wedge) =====
// Slots 0,1,2 = bottom triangle; 3,4,5 = top triangle (slots 3,4,5 align with 0,1,2).
const PRISM_FACES: &[FaceData] = &[
    FaceData { polygon: &[0, 2, 1],    triangulation: &[[0, 2, 1]] },                   // Bottom tri
    FaceData { polygon: &[0, 3, 5, 2], triangulation: &[[0, 5, 2], [0, 3, 5]] },        // Side quad 1
    FaceData { polygon: &[2, 5, 4, 1], triangulation: &[[2, 4, 5], [2, 1, 4]] },        // Side quad 2
    FaceData { polygon: &[0, 1, 4, 3], triangulation: &[[3, 0, 4], [0, 1, 4]] },        // Side quad 3
    FaceData { polygon: &[3, 4, 5],    triangulation: &[[3, 4, 5]] },                   // Top tri
];

// ===== Pyramid =====
// Slots 0..3 = base quad (CCW from outside, looking from -apex toward base);
// Slot 4 = apex.
const PYRAMID_FACES: &[FaceData] = &[
    FaceData { polygon: &[0, 1, 2, 3], triangulation: &[[0, 3, 2], [0, 2, 1]] }, // Base quad
    FaceData { polygon: &[0, 1, 4],    triangulation: &[[0, 1, 4]] },            // Side 1
    FaceData { polygon: &[1, 2, 4],    triangulation: &[[1, 2, 4]] },            // Side 2
    FaceData { polygon: &[2, 3, 4],    triangulation: &[[2, 3, 4]] },            // Side 3
    FaceData { polygon: &[3, 0, 4],    triangulation: &[[3, 0, 4]] },            // Side 4
];

/// Returns the face data table for a given cell type.
#[must_use]
pub fn face_data_for(cell_type: VolumeCellType) -> &'static [FaceData] {
    match cell_type {
        VolumeCellType::Tet => TET_FACES,
        VolumeCellType::Hex => HEX_FACES,
        VolumeCellType::Prism => PRISM_FACES,
        VolumeCellType::Pyramid => PYRAMID_FACES,
    }
}

/// Number of vertices used by a cell type (non-sentinel slots in the `[u32; 8]`).
#[must_use]
pub fn num_verts_in_cell(cell_type: VolumeCellType) -> usize {
    match cell_type {
        VolumeCellType::Tet => 4,
        VolumeCellType::Hex => 8,
        VolumeCellType::Prism => 6,
        VolumeCellType::Pyramid => 5,
    }
}

/// Builds a canonical (sorted) face key for hashing.
///
/// Face polygons can have 3 or 4 unique vertices. Triangular faces leave slot 3
/// as `u32::MAX` so that triangle and quad keys never collide.
#[must_use]
pub fn canonical_face_key(cell: &[u32; 8], polygon: &[usize]) -> [u32; 4] {
    let mut key = [u32::MAX; 4];
    debug_assert!(polygon.len() == 3 || polygon.len() == 4);
    for (i, &slot) in polygon.iter().enumerate() {
        key[i] = cell[slot];
    }
    key.sort_unstable();
    key
}
```

Then in `crates/polyscope-structures/src/volume_mesh/mod.rs`, add the submodule declaration. Look for the existing `mod ...;` declarations near line 44, and add:

```rust
mod cell_data;
```

between `mod color_quantity;` and `mod scalar_quantity;`. Keep alphabetical order if other declarations follow it.

- [ ] **Step 5: Run test to verify it passes**

Run: `cargo test -p polyscope-structures test_cell_type_enum_has_prism_and_pyramid -- --nocapture`
Expected: PASS.

- [ ] **Step 6: Run clippy**

Run: `cargo clippy --workspace -- -D warnings`
Expected: Zero warnings.

- [ ] **Step 7: Commit**

```bash
git add crates/polyscope-structures/src/volume_mesh/cell_data.rs crates/polyscope-structures/src/volume_mesh/mod.rs
git commit -m "feat(volume_mesh): scaffold prism/pyramid cell types and shape data"
```

---

### Task 2: Switch `cell_type()` to sentinel-count detection

**Files:**
- Modify: `crates/polyscope-structures/src/volume_mesh/mod.rs:189-197` (the `cell_type` method)

- [ ] **Step 1: Write the failing tests**

Append to the `#[cfg(test)] mod tests` block:

```rust
#[test]
fn test_cell_type_detection_tet() {
    let mesh = VolumeMesh::new(
        "t",
        vec![Vec3::ZERO, Vec3::X, Vec3::Y, Vec3::Z],
        vec![[0, 1, 2, 3, u32::MAX, u32::MAX, u32::MAX, u32::MAX]],
    );
    assert_eq!(mesh.cell_type(0), VolumeCellType::Tet);
}

#[test]
fn test_cell_type_detection_hex() {
    // 8 distinct vertices, no sentinels
    let verts = (0..8).map(|i| Vec3::splat(i as f32)).collect();
    let mesh = VolumeMesh::new("h", verts, vec![[0, 1, 2, 3, 4, 5, 6, 7]]);
    assert_eq!(mesh.cell_type(0), VolumeCellType::Hex);
}

#[test]
fn test_cell_type_detection_prism() {
    // 6 verts in slots 0..5, two sentinels in slots 6..7
    let verts = (0..6).map(|i| Vec3::splat(i as f32)).collect();
    let mesh = VolumeMesh::new(
        "p",
        verts,
        vec![[0, 1, 2, 3, 4, 5, u32::MAX, u32::MAX]],
    );
    assert_eq!(mesh.cell_type(0), VolumeCellType::Prism);
}

#[test]
fn test_cell_type_detection_pyramid() {
    // 5 verts in slots 0..4, three sentinels in slots 5..7
    let verts = (0..5).map(|i| Vec3::splat(i as f32)).collect();
    let mesh = VolumeMesh::new(
        "py",
        verts,
        vec![[0, 1, 2, 3, 4, u32::MAX, u32::MAX, u32::MAX]],
    );
    assert_eq!(mesh.cell_type(0), VolumeCellType::Pyramid);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p polyscope-structures test_cell_type_detection -- --nocapture`
Expected: FAIL — prism and pyramid currently misclassified as `Hex` (any non-tet returns `Hex` under the old check).

- [ ] **Step 3: Replace the detection implementation**

In `crates/polyscope-structures/src/volume_mesh/mod.rs`, replace the `cell_type` method body (currently lines 189-197):

```rust
    /// Returns the cell type of the given cell.
    #[must_use]
    pub fn cell_type(&self, cell_idx: usize) -> VolumeCellType {
        if self.cells[cell_idx][4] == u32::MAX {
            VolumeCellType::Tet
        } else {
            VolumeCellType::Hex
        }
    }
```

with:

```rust
    /// Returns the cell type of the given cell.
    ///
    /// Cell type is determined by the number of sentinel (`u32::MAX`) indices
    /// in the 8-slot cell array (matches upstream C++ Polyscope):
    /// - 0 sentinels → `Hex` (8 verts)
    /// - 2 sentinels → `Prism` (6 verts)
    /// - 3 sentinels → `Pyramid` (5 verts)
    /// - 4 sentinels → `Tet` (4 verts)
    ///
    /// # Panics
    /// Panics if `cell_idx` is out of range or the sentinel count is invalid
    /// (1, 5, 6, 7, or 8 sentinels).
    #[must_use]
    pub fn cell_type(&self, cell_idx: usize) -> VolumeCellType {
        let sentinels = self.cells[cell_idx]
            .iter()
            .filter(|&&v| v == u32::MAX)
            .count();
        match sentinels {
            0 => VolumeCellType::Hex,
            2 => VolumeCellType::Prism,
            3 => VolumeCellType::Pyramid,
            4 => VolumeCellType::Tet,
            n => panic!("VolumeMesh cell {cell_idx}: invalid sentinel count {n} (expected 0/2/3/4)"),
        }
    }
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p polyscope-structures test_cell_type_detection -- --nocapture`
Expected: All 4 PASS.

- [ ] **Step 5: Re-run the full structure test suite to check for regressions**

Run: `cargo test -p polyscope-structures -- --nocapture`
Expected: All existing tests still PASS.

- [ ] **Step 6: Run clippy**

Run: `cargo clippy --workspace -- -D warnings`
Expected: Zero warnings.

- [ ] **Step 7: Commit**

```bash
git add crates/polyscope-structures/src/volume_mesh/mod.rs
git commit -m "feat(volume_mesh): use sentinel-count to classify cell type"
```

---

### Task 3: Wire prism / pyramid into `decompose_to_tets` and `cell_centroid`

**Files:**
- Modify: `crates/polyscope-structures/src/volume_mesh/mod.rs:259-336` (decompose_to_tets, cell_centroid)
- Modify: `crates/polyscope-structures/src/volume_mesh/cell_data.rs` (add tet-decomposition helpers)

- [ ] **Step 1: Write the failing tests**

Append to the test module:

```rust
#[test]
fn test_decompose_prism_to_tets() {
    // Unit triangular prism: bottom tri at z=0, top tri at z=1
    let verts = vec![
        Vec3::new(0.0, 0.0, 0.0),  // 0
        Vec3::new(1.0, 0.0, 0.0),  // 1
        Vec3::new(0.5, 1.0, 0.0),  // 2
        Vec3::new(0.0, 0.0, 1.0),  // 3
        Vec3::new(1.0, 0.0, 1.0),  // 4
        Vec3::new(0.5, 1.0, 1.0),  // 5
    ];
    let mesh = VolumeMesh::new(
        "prism_only",
        verts,
        vec![[0, 1, 2, 3, 4, 5, u32::MAX, u32::MAX]],
    );
    let tets = mesh.decompose_to_tets();
    // A prism decomposes into exactly 3 tetrahedra
    assert_eq!(tets.len(), 3, "prism should decompose to 3 tets");
    for tet in &tets {
        for &v in tet {
            assert!(v < 6, "tet vertex index {v} out of range for prism");
        }
    }
}

#[test]
fn test_decompose_pyramid_to_tets() {
    let verts = vec![
        Vec3::new(0.0, 0.0, 0.0),  // 0
        Vec3::new(1.0, 0.0, 0.0),  // 1
        Vec3::new(1.0, 1.0, 0.0),  // 2
        Vec3::new(0.0, 1.0, 0.0),  // 3
        Vec3::new(0.5, 0.5, 1.0),  // 4 (apex)
    ];
    let mesh = VolumeMesh::new(
        "pyr_only",
        verts,
        vec![[0, 1, 2, 3, 4, u32::MAX, u32::MAX, u32::MAX]],
    );
    let tets = mesh.decompose_to_tets();
    // A pyramid decomposes into exactly 2 tetrahedra
    assert_eq!(tets.len(), 2, "pyramid should decompose to 2 tets");
    for tet in &tets {
        for &v in tet {
            assert!(v < 5, "tet vertex index {v} out of range for pyramid");
        }
    }
}

#[test]
fn test_cell_centroid_prism() {
    // Build a prism with known centroid
    let verts = vec![
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(2.0, 0.0, 0.0),
        Vec3::new(1.0, 2.0, 0.0),
        Vec3::new(0.0, 0.0, 4.0),
        Vec3::new(2.0, 0.0, 4.0),
        Vec3::new(1.0, 2.0, 4.0),
    ];
    let mesh = VolumeMesh::new(
        "p", verts.clone(),
        vec![[0, 1, 2, 3, 4, 5, u32::MAX, u32::MAX]],
    );
    // Centroid is mean of the 6 vertices
    let expected: Vec3 = verts.iter().copied().sum::<Vec3>() / 6.0;
    let centroid = mesh.cell_centroid(&mesh.cells()[0]);
    assert!((centroid - expected).length() < 1e-5);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p polyscope-structures test_decompose_prism_to_tets test_decompose_pyramid_to_tets test_cell_centroid_prism -- --nocapture`
Expected: FAIL — `decompose_to_tets` currently only handles tet/hex (returns 0 tets for prism/pyramid) and `cell_centroid` averages 8 vertices for non-tet cells (incorrect for prism/pyramid since slots 6-7 hold `u32::MAX`).

- [ ] **Step 3: Add tet-decomposition helpers to `cell_data.rs`**

Append to `crates/polyscope-structures/src/volume_mesh/cell_data.rs`:

```rust
/// Decomposes a hex cell into 5 tetrahedra using a fixed diagonal pattern.
///
/// This matches the existing polyscope-rs decomposition (the central-diagonal
/// pattern from Dompierre et al.). The 5-tet split works for any convex hex.
const HEX_TO_TET_PATTERN: [[usize; 4]; 5] = [
    [0, 1, 2, 5],
    [0, 2, 7, 5],
    [0, 2, 3, 7],
    [0, 5, 7, 4],
    [2, 7, 5, 6],
];

/// Decomposes a triangular prism into 3 tetrahedra.
///
/// Picks a consistent diagonal split on the quad face opposite the
/// lowest-numbered vertex, matching the upstream algorithm (decomposePrism in
/// polyscope/src/volume_mesh.cpp). Consistency across adjacent cells matters
/// for mixed meshes so that shared faces are tessellated identically and no
/// gaps appear in slice caps.
pub fn decompose_prism(cell: &[u32; 8]) -> [[u32; 4]; 3] {
    let mut p: [u32; 6] = [cell[0], cell[1], cell[2], cell[3], cell[4], cell[5]];

    // Find index of smallest vertex
    let min_idx = (0..6).min_by_key(|&i| p[i]).unwrap();

    if min_idx < 3 {
        // Smallest vertex is in the bottom triangle. Rotate bottom so it lands in slot 0.
        let rot = match min_idx {
            0 => 0,
            1 => 2,
            _ => 1, // min_idx == 2
        };
        rotate_prism_in_place(&mut p, rot);
    } else {
        // Smallest is in the top triangle. Reflect top<->bottom, then rotate.
        let top_pos = min_idx - 3;
        let rot = match top_pos {
            0 => 0,
            1 => 2,
            _ => 1, // top_pos == 2
        };
        p.swap(0, 3);
        p.swap(1, 4);
        p.swap(2, 5);
        rotate_prism_in_place(&mut p, rot);
    }

    // Split the quad opposite V0 along the diagonal containing the smaller of
    // {p[2], p[4]} vs {p[1], p[5]}, so adjacent cells choose the same split.
    if p[2].min(p[4]) < p[1].min(p[5]) {
        // Diagonal 2-4
        [
            [p[0], p[5], p[4], p[3]],
            [p[0], p[4], p[5], p[2]],
            [p[0], p[4], p[2], p[1]],
        ]
    } else {
        // Diagonal 1-5
        [
            [p[0], p[5], p[4], p[3]],
            [p[0], p[1], p[5], p[2]],
            [p[0], p[5], p[1], p[4]],
        ]
    }
}

fn rotate_prism_in_place(p: &mut [u32; 6], rot: usize) {
    const BOTTOM_ROT: [[usize; 3]; 3] = [[0, 1, 2], [1, 2, 0], [2, 0, 1]];
    const TOP_ROT: [[usize; 3]; 3] = [[3, 4, 5], [4, 5, 3], [5, 3, 4]];
    let src = *p;
    for i in 0..3 {
        p[i] = src[BOTTOM_ROT[rot][i]];
        p[i + 3] = src[TOP_ROT[rot][i]];
    }
}

/// Decomposes a square pyramid into 2 tetrahedra by splitting the base quad
/// along the diagonal containing the smaller of {p[0], p[2]} vs {p[1], p[3]}.
/// Consistent split ensures adjacent cells tessellate the shared face the same way.
pub fn decompose_pyramid(cell: &[u32; 8]) -> [[u32; 4]; 2] {
    let p: [u32; 5] = [cell[0], cell[1], cell[2], cell[3], cell[4]];

    if p[0].min(p[2]) < p[1].min(p[3]) {
        // Diagonal 0-2
        [
            [p[0], p[2], p[4], p[1]],
            [p[0], p[4], p[2], p[3]],
        ]
    } else {
        // Diagonal 1-3
        [
            [p[1], p[3], p[4], p[2]],
            [p[1], p[4], p[3], p[0]],
        ]
    }
}

/// Returns the tet-decomposition for any cell, dispatching on cell type.
pub fn decompose_cell_to_tets(cell: &[u32; 8], cell_type: VolumeCellType) -> Vec<[u32; 4]> {
    match cell_type {
        VolumeCellType::Tet => vec![[cell[0], cell[1], cell[2], cell[3]]],
        VolumeCellType::Hex => HEX_TO_TET_PATTERN
            .iter()
            .map(|t| [cell[t[0]], cell[t[1]], cell[t[2]], cell[t[3]]])
            .collect(),
        VolumeCellType::Prism => decompose_prism(cell).to_vec(),
        VolumeCellType::Pyramid => decompose_pyramid(cell).to_vec(),
    }
}
```

- [ ] **Step 4: Replace `decompose_to_tets` in `mod.rs`**

In `crates/polyscope-structures/src/volume_mesh/mod.rs`, replace `decompose_to_tets` (currently lines 259-284):

```rust
    /// Decomposes all cells into tetrahedra.
    /// Tets pass through unchanged, hexes are decomposed into 5 tets.
    #[must_use]
    pub fn decompose_to_tets(&self) -> Vec<[u32; 4]> {
        let mut tets = Vec::new();

        for cell in &self.cells {
            if cell[4] == u32::MAX {
                // Already a tet
                tets.push([cell[0], cell[1], cell[2], cell[3]]);
            } else {
                // Hex - decompose using diagonal pattern (5 tets)
                for tet_local in &HEX_TO_TET_PATTERN {
                    let tet = [
                        cell[tet_local[0]],
                        cell[tet_local[1]],
                        cell[tet_local[2]],
                        cell[tet_local[3]],
                    ];
                    tets.push(tet);
                }
            }
        }

        tets
    }
```

with:

```rust
    /// Decomposes all cells into tetrahedra.
    ///
    /// Decomposition counts per cell type:
    /// - Tet → 1 tet (passthrough)
    /// - Hex → 5 tets (fixed diagonal pattern)
    /// - Prism → 3 tets (consistent diagonal split)
    /// - Pyramid → 2 tets (consistent diagonal split)
    #[must_use]
    pub fn decompose_to_tets(&self) -> Vec<[u32; 4]> {
        let mut tets = Vec::new();
        for (cell_idx, cell) in self.cells.iter().enumerate() {
            let ct = self.cell_type(cell_idx);
            tets.extend(cell_data::decompose_cell_to_tets(cell, ct));
        }
        tets
    }
```

Add `use cell_data;` (or `use super::cell_data;` depending on context) near the top of `mod.rs` — check the existing `use` statements around lines 56-62. If those are absolute paths, just refer to `cell_data::...` directly since the submodule is in scope.

- [ ] **Step 5: Replace `cell_centroid` in `mod.rs`**

In `crates/polyscope-structures/src/volume_mesh/mod.rs`, replace `cell_centroid` (currently lines 320-336):

```rust
    /// Computes the centroid of a cell.
    fn cell_centroid(&self, cell: &[u32; 8]) -> Vec3 {
        if cell[4] == u32::MAX {
            // Tetrahedron: average of 4 vertices
            let sum = self.vertices[cell[0] as usize]
                + self.vertices[cell[1] as usize]
                + self.vertices[cell[2] as usize]
                + self.vertices[cell[3] as usize];
            sum / 4.0
        } else {
            // Hexahedron: average of 8 vertices
            let sum = (0..8)
                .map(|i| self.vertices[cell[i] as usize])
                .fold(Vec3::ZERO, |a, b| a + b);
            sum / 8.0
        }
    }
```

with:

```rust
    /// Computes the centroid of a cell as the mean of its real (non-sentinel) vertices.
    pub(crate) fn cell_centroid(&self, cell: &[u32; 8]) -> Vec3 {
        let mut sum = Vec3::ZERO;
        let mut count = 0u32;
        for &v in cell {
            if v != u32::MAX {
                sum += self.vertices[v as usize];
                count += 1;
            }
        }
        sum / count as f32
    }
```

(Note the visibility change to `pub(crate)` to allow the test to call it directly. If the existing visibility was private, leave it `fn cell_centroid` and instead expose a test helper or use a non-direct-access test; otherwise `pub(crate)` is fine for crate-internal tests in the same module.)

- [ ] **Step 6: Delete the now-orphaned `HEX_TO_TET_PATTERN` constant in `mod.rs`**

In `crates/polyscope-structures/src/volume_mesh/mod.rs`, delete the constant block at line 1442-1449:

```rust
/// Diagonal decomposition patterns (5 tets).
const HEX_TO_TET_PATTERN: [[usize; 4]; 5] = [
    [0, 1, 2, 5],
    [0, 2, 7, 5],
    [0, 2, 3, 7],
    [0, 5, 7, 4],
    [2, 7, 5, 6],
];
```

It now lives in `cell_data.rs`.

- [ ] **Step 7: Run tests to verify they pass**

Run: `cargo test -p polyscope-structures test_decompose_prism_to_tets test_decompose_pyramid_to_tets test_cell_centroid_prism -- --nocapture`
Expected: PASS.

Run the existing hex/tet tests too:

Run: `cargo test -p polyscope-structures test_hex_to_tet_decomposition test_single_tet_all_exterior -- --nocapture`
Expected: PASS (regression check).

- [ ] **Step 8: Run clippy**

Run: `cargo clippy --workspace -- -D warnings`
Expected: Zero warnings.

- [ ] **Step 9: Commit**

```bash
git add crates/polyscope-structures/src/volume_mesh/mod.rs crates/polyscope-structures/src/volume_mesh/cell_data.rs
git commit -m "feat(volume_mesh): tet-decomposition + centroid for prism/pyramid"
```

---

### Task 4: Refactor face-counting and render geometry to dispatch through `cell_data`

This task consolidates the six near-duplicate per-cell loops (`compute_face_counts`, `compute_face_counts_with_culling`, `generate_render_geometry`, `generate_render_geometry_with_culling`, `generate_render_geometry_with_quantities`, `generate_cell_index_per_triangle`, `generate_cell_index_per_triangle_with_culling`) so each handles all 4 cell types uniformly.

**Files:**
- Modify: `crates/polyscope-structures/src/volume_mesh/mod.rs:292-829` (the seven functions listed above)

- [ ] **Step 1: Write the failing tests**

Append to the test module:

```rust
#[test]
fn test_single_prism_all_exterior() {
    let verts = vec![
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(1.0, 0.0, 0.0),
        Vec3::new(0.5, 1.0, 0.0),
        Vec3::new(0.0, 0.0, 1.0),
        Vec3::new(1.0, 0.0, 1.0),
        Vec3::new(0.5, 1.0, 1.0),
    ];
    let mesh = VolumeMesh::new(
        "p", verts,
        vec![[0, 1, 2, 3, 4, 5, u32::MAX, u32::MAX]],
    );
    let (_, faces) = mesh.generate_render_geometry();
    // Prism has 5 faces total: 2 tris + 3 quads = 8 triangles after triangulation
    assert_eq!(faces.len(), 8, "single prism should have 8 triangles (2 tri + 3*2 quad)");
}

#[test]
fn test_single_pyramid_all_exterior() {
    let verts = vec![
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(1.0, 0.0, 0.0),
        Vec3::new(1.0, 1.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
        Vec3::new(0.5, 0.5, 1.0),
    ];
    let mesh = VolumeMesh::new(
        "py", verts,
        vec![[0, 1, 2, 3, 4, u32::MAX, u32::MAX, u32::MAX]],
    );
    let (_, faces) = mesh.generate_render_geometry();
    // Pyramid: 1 quad base + 4 tri sides = 2 + 4 = 6 triangles
    assert_eq!(faces.len(), 6, "single pyramid should have 6 triangles (2 base + 4 sides)");
}

#[test]
fn test_mixed_cell_mesh_renders_all() {
    // One tet + one prism + one pyramid sharing zero faces -> all faces exterior
    let verts = vec![
        // Tet
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(1.0, 0.0, 0.0),
        Vec3::new(0.5, 1.0, 0.0),
        Vec3::new(0.5, 0.5, 1.0),
        // Prism (translated +5 in x)
        Vec3::new(5.0, 0.0, 0.0),
        Vec3::new(6.0, 0.0, 0.0),
        Vec3::new(5.5, 1.0, 0.0),
        Vec3::new(5.0, 0.0, 1.0),
        Vec3::new(6.0, 0.0, 1.0),
        Vec3::new(5.5, 1.0, 1.0),
        // Pyramid (translated +10 in x)
        Vec3::new(10.0, 0.0, 0.0),
        Vec3::new(11.0, 0.0, 0.0),
        Vec3::new(11.0, 1.0, 0.0),
        Vec3::new(10.0, 1.0, 0.0),
        Vec3::new(10.5, 0.5, 1.0),
    ];
    let cells = vec![
        [0, 1, 2, 3, u32::MAX, u32::MAX, u32::MAX, u32::MAX], // tet (4 tris)
        [4, 5, 6, 7, 8, 9, u32::MAX, u32::MAX],               // prism (8 tris)
        [10, 11, 12, 13, 14, u32::MAX, u32::MAX, u32::MAX],   // pyramid (6 tris)
    ];
    let mesh = VolumeMesh::new("m", verts, cells);
    let (_, faces) = mesh.generate_render_geometry();
    assert_eq!(faces.len(), 4 + 8 + 6, "mixed mesh should sum per-cell triangle counts");
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p polyscope-structures test_single_prism_all_exterior test_single_pyramid_all_exterior test_mixed_cell_mesh_renders_all -- --nocapture`
Expected: FAIL — prism/pyramid currently go down the `else` (hex) branch and emit 12 triangles each.

- [ ] **Step 3: Replace `compute_face_counts`**

In `crates/polyscope-structures/src/volume_mesh/mod.rs`, replace `compute_face_counts` (currently lines 293-318):

```rust
    /// Computes face counts for interior/exterior detection.
    fn compute_face_counts(&self) -> HashMap<[u32; 4], usize> {
        let mut face_counts: HashMap<[u32; 4], usize> = HashMap::new();
        for (cell_idx, cell) in self.cells.iter().enumerate() {
            let ct = self.cell_type(cell_idx);
            for face in cell_data::face_data_for(ct) {
                let key = cell_data::canonical_face_key(cell, face.polygon);
                *face_counts.entry(key).or_insert(0) += 1;
            }
        }
        face_counts
    }
```

- [ ] **Step 4: Replace `compute_face_counts_with_culling`**

Replace the function at lines 356-389 with:

```rust
    /// Same as `compute_face_counts` but skips cells culled by slice planes.
    fn compute_face_counts_with_culling(
        &self,
        planes: &[(Vec3, Vec3)],
    ) -> HashMap<[u32; 4], usize> {
        let mut face_counts: HashMap<[u32; 4], usize> = HashMap::new();
        for (cell_idx, cell) in self.cells.iter().enumerate() {
            if !self.is_cell_visible(cell, planes) {
                continue;
            }
            let ct = self.cell_type(cell_idx);
            for face in cell_data::face_data_for(ct) {
                let key = cell_data::canonical_face_key(cell, face.polygon);
                *face_counts.entry(key).or_insert(0) += 1;
            }
        }
        face_counts
    }
```

- [ ] **Step 5: Replace `generate_render_geometry`**

Replace the function at lines 391-434 with:

```rust
    /// Generates triangulated exterior faces for rendering.
    fn generate_render_geometry(&self) -> (Vec<Vec3>, Vec<[u32; 3]>) {
        let face_counts = self.compute_face_counts();
        let mut positions = Vec::new();
        let mut faces = Vec::new();

        for (cell_idx, cell) in self.cells.iter().enumerate() {
            let ct = self.cell_type(cell_idx);
            for face in cell_data::face_data_for(ct) {
                let key = cell_data::canonical_face_key(cell, face.polygon);
                if face_counts[&key] != 1 {
                    continue;
                }
                for &[a, b, c] in face.triangulation {
                    let base_idx = positions.len() as u32;
                    positions.push(self.vertices[cell[a] as usize]);
                    positions.push(self.vertices[cell[b] as usize]);
                    positions.push(self.vertices[cell[c] as usize]);
                    faces.push([base_idx, base_idx + 1, base_idx + 2]);
                }
            }
        }

        (positions, faces)
    }
```

- [ ] **Step 6: Replace `generate_render_geometry_with_culling`**

Replace the function at lines 436-488 with:

```rust
    /// Generates triangulated exterior faces with cell culling based on slice planes.
    fn generate_render_geometry_with_culling(
        &self,
        planes: &[(Vec3, Vec3)],
    ) -> (Vec<Vec3>, Vec<[u32; 3]>) {
        let face_counts = self.compute_face_counts_with_culling(planes);
        let mut positions = Vec::new();
        let mut faces = Vec::new();

        for (cell_idx, cell) in self.cells.iter().enumerate() {
            if !self.is_cell_visible(cell, planes) {
                continue;
            }
            let ct = self.cell_type(cell_idx);
            for face in cell_data::face_data_for(ct) {
                let key = cell_data::canonical_face_key(cell, face.polygon);
                if face_counts.get(&key) != Some(&1) {
                    continue;
                }
                for &[a, b, c] in face.triangulation {
                    let base_idx = positions.len() as u32;
                    positions.push(self.vertices[cell[a] as usize]);
                    positions.push(self.vertices[cell[b] as usize]);
                    positions.push(self.vertices[cell[c] as usize]);
                    faces.push([base_idx, base_idx + 1, base_idx + 2]);
                }
            }
        }

        (positions, faces)
    }
```

- [ ] **Step 7: Replace `generate_render_geometry_with_quantities` per-cell loop**

The body of this function spans roughly lines 492-... and has two passes (geometry + normals + per-vertex quantity values). The first pass iterates cells; replace the per-cell `if cell[4] == u32::MAX { tet branch } else { hex branch }` block with:

```rust
            let ct = self.cell_type(cell_idx);
            for face in cell_data::face_data_for(ct) {
                let key = cell_data::canonical_face_key(cell, face.polygon);
                if face_counts[&key] != 1 {
                    continue;
                }
                for &[a, b, c] in face.triangulation {
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
```

Read the surrounding context with `cargo run -- ` or by opening the file: confirm that `vertex_indices`, `cell_indices`, `positions`, `faces`, and `face_counts` are the bindings in scope and adjust names if any differ. The rest of the function (normals computation, quantity sampling) does not depend on cell type and stays unchanged.

- [ ] **Step 8: Replace `generate_cell_index_per_triangle` and `generate_cell_index_per_triangle_with_culling`**

Replace the body of each function with the same dispatch pattern. For `generate_cell_index_per_triangle` (around lines 763-794):

```rust
    fn generate_cell_index_per_triangle(&self) -> Vec<u32> {
        let face_counts = self.compute_face_counts();
        let mut cell_indices = Vec::new();
        for (cell_idx, cell) in self.cells.iter().enumerate() {
            let ct = self.cell_type(cell_idx);
            for face in cell_data::face_data_for(ct) {
                let key = cell_data::canonical_face_key(cell, face.polygon);
                if face_counts.get(&key) != Some(&1) {
                    continue;
                }
                for _tri in face.triangulation {
                    cell_indices.push(cell_idx as u32);
                }
            }
        }
        cell_indices
    }
```

And the `_with_culling` variant (around lines 796-829):

```rust
    fn generate_cell_index_per_triangle_with_culling(&self, planes: &[(Vec3, Vec3)]) -> Vec<u32> {
        let face_counts = self.compute_face_counts_with_culling(planes);
        let mut cell_indices = Vec::new();
        for (cell_idx, cell) in self.cells.iter().enumerate() {
            if !self.is_cell_visible(cell, planes) {
                continue;
            }
            let ct = self.cell_type(cell_idx);
            for face in cell_data::face_data_for(ct) {
                let key = cell_data::canonical_face_key(cell, face.polygon);
                if face_counts.get(&key) != Some(&1) {
                    continue;
                }
                for _tri in face.triangulation {
                    cell_indices.push(cell_idx as u32);
                }
            }
        }
        cell_indices
    }
```

- [ ] **Step 9: Delete the now-orphaned constants in `mod.rs`**

In `crates/polyscope-structures/src/volume_mesh/mod.rs`, delete these blocks (now in `cell_data.rs`):

- `canonical_face_key` (lines ~1410-1416)
- `TET_FACE_STENCIL` (line ~1419)
- `HEX_FACE_STENCIL` (lines ~1421-1429)

The `VolumeMeshRenderGeometry` struct (lines ~1431-1440) stays.

- [ ] **Step 10: Run tests**

Run: `cargo test -p polyscope-structures -- --nocapture`
Expected: All tests PASS — both the new prism/pyramid/mixed tests and every existing tet/hex test (regression check).

- [ ] **Step 11: Build the full workspace and run clippy**

Run: `cargo build --workspace`
Expected: Builds clean.

Run: `cargo clippy --workspace -- -D warnings`
Expected: Zero warnings.

- [ ] **Step 12: Commit**

```bash
git add crates/polyscope-structures/src/volume_mesh/mod.rs crates/polyscope-structures/src/volume_mesh/cell_data.rs
git commit -m "refactor(volume_mesh): dispatch face iteration via cell_data table"
```

---

### Task 5: Slice geometry for prism and pyramid

**Files:**
- Modify: `crates/polyscope-structures/src/volume_mesh/slice_geometry.rs` (add two new functions)
- Modify: `crates/polyscope-structures/src/volume_mesh/mod.rs` (update the `pub use slice_geometry::...` re-export)

- [ ] **Step 1: Write the failing tests**

Append to the `#[cfg(test)] mod tests` block in `slice_geometry.rs`:

```rust
#[test]
fn test_slice_prism_through_middle() {
    // Unit triangular prism z ∈ [0,1]; slice horizontally at y=... no, let's use z=0.5
    let verts = [
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(1.0, 0.0, 0.0),
        Vec3::new(0.5, 1.0, 0.0),
        Vec3::new(0.0, 0.0, 1.0),
        Vec3::new(1.0, 0.0, 1.0),
        Vec3::new(0.5, 1.0, 1.0),
    ];
    let result = slice_prism(verts, Vec3::new(0.0, 0.0, 0.5), Vec3::Z);
    assert!(result.has_intersection());
    // Slice through the prism mid-height should yield a triangular cross-section
    assert!(result.vertices.len() >= 3, "expected at least 3 verts, got {}", result.vertices.len());
    for v in &result.vertices {
        assert!((v.z - 0.5).abs() < 1e-4, "vertex z={} should be 0.5", v.z);
    }
}

#[test]
fn test_slice_prism_no_intersection() {
    let verts = [
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(1.0, 0.0, 0.0),
        Vec3::new(0.5, 1.0, 0.0),
        Vec3::new(0.0, 0.0, 1.0),
        Vec3::new(1.0, 0.0, 1.0),
        Vec3::new(0.5, 1.0, 1.0),
    ];
    // Plane above the prism
    let result = slice_prism(verts, Vec3::new(0.0, 0.0, 2.0), Vec3::Z);
    assert!(!result.has_intersection());
}

#[test]
fn test_slice_pyramid_through_middle() {
    // Unit pyramid: base z=0, apex at (0.5,0.5,1)
    let verts = [
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(1.0, 0.0, 0.0),
        Vec3::new(1.0, 1.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
        Vec3::new(0.5, 0.5, 1.0),
    ];
    let result = slice_pyramid(verts, Vec3::new(0.0, 0.0, 0.5), Vec3::Z);
    assert!(result.has_intersection());
    // Slice at half height yields a square cross-section
    assert!(result.vertices.len() >= 3);
    for v in &result.vertices {
        assert!((v.z - 0.5).abs() < 1e-4);
    }
}

#[test]
fn test_slice_pyramid_no_intersection() {
    let verts = [
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(1.0, 0.0, 0.0),
        Vec3::new(1.0, 1.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
        Vec3::new(0.5, 0.5, 1.0),
    ];
    let result = slice_pyramid(verts, Vec3::new(0.0, 0.0, -1.0), Vec3::Z);
    assert!(!result.has_intersection());
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p polyscope-structures slice_prism slice_pyramid -- --nocapture`
Expected: FAIL — `slice_prism` / `slice_pyramid` not yet defined.

- [ ] **Step 3: Implement `slice_prism`**

In `crates/polyscope-structures/src/volume_mesh/slice_geometry.rs`, append after `slice_hex` (after line 149):

```rust
/// Slice a triangular prism by decomposing into 3 tetrahedra.
///
/// # Arguments
/// * `vertices` - The 6 vertices of the prism (slots 0..2 = bottom tri, 3..5 = top tri)
/// * `plane_origin` - A point on the plane
/// * `plane_normal` - The plane normal (points toward kept geometry)
#[must_use]
pub fn slice_prism(vertices: [Vec3; 6], plane_origin: Vec3, plane_normal: Vec3) -> CellSliceResult {
    // Symmetric 3-tet decomposition. Matches the upstream prism split for V0
    // in the bottom-left corner with no rotation (default case): tets
    //   {0, 5, 4, 3}, {0, 4, 5, 2}, {0, 4, 2, 1}
    // (See cell_data::decompose_prism for the consistent variant; here we don't
    // care about cross-cell consistency because slicing operates on isolated cells.)
    let tet_indices = [
        [0usize, 5, 4, 3],
        [0, 4, 5, 2],
        [0, 4, 2, 1],
    ];

    let mut all_vertices = Vec::new();
    let mut all_interp = Vec::new();

    for tet in &tet_indices {
        let r = slice_tet(
            vertices[tet[0]],
            vertices[tet[1]],
            vertices[tet[2]],
            vertices[tet[3]],
            plane_origin,
            plane_normal,
        );
        for (local_a, local_b, t) in r.interpolation {
            let pa = tet[local_a as usize] as u32;
            let pb = tet[local_b as usize] as u32;
            all_interp.push((pa, pb, t));
        }
        all_vertices.extend(r.vertices);
    }

    merge_slice_vertices(&mut all_vertices, &mut all_interp);
    if all_vertices.len() >= 3 {
        order_polygon_vertices(&mut all_vertices, &mut all_interp, plane_normal);
    }

    CellSliceResult { vertices: all_vertices, interpolation: all_interp }
}

/// Slice a square pyramid by decomposing into 2 tetrahedra.
///
/// # Arguments
/// * `vertices` - The 5 vertices of the pyramid (slots 0..3 = base quad, 4 = apex)
/// * `plane_origin` - A point on the plane
/// * `plane_normal` - The plane normal (points toward kept geometry)
#[must_use]
pub fn slice_pyramid(vertices: [Vec3; 5], plane_origin: Vec3, plane_normal: Vec3) -> CellSliceResult {
    // Split base quad along diagonal 0-2 (consistent with cell_data::decompose_pyramid
    // when p[0].min(p[2]) is the smaller pair). For isolated slicing the split
    // choice doesn't affect correctness.
    let tet_indices = [
        [0usize, 2, 4, 1],
        [0, 4, 2, 3],
    ];

    let mut all_vertices = Vec::new();
    let mut all_interp = Vec::new();

    for tet in &tet_indices {
        let r = slice_tet(
            vertices[tet[0]],
            vertices[tet[1]],
            vertices[tet[2]],
            vertices[tet[3]],
            plane_origin,
            plane_normal,
        );
        for (local_a, local_b, t) in r.interpolation {
            let pa = tet[local_a as usize] as u32;
            let pb = tet[local_b as usize] as u32;
            all_interp.push((pa, pb, t));
        }
        all_vertices.extend(r.vertices);
    }

    merge_slice_vertices(&mut all_vertices, &mut all_interp);
    if all_vertices.len() >= 3 {
        order_polygon_vertices(&mut all_vertices, &mut all_interp, plane_normal);
    }

    CellSliceResult { vertices: all_vertices, interpolation: all_interp }
}
```

- [ ] **Step 4: Re-export the new functions**

In `crates/polyscope-structures/src/volume_mesh/mod.rs`, find the existing line (around line 51):

```rust
pub use slice_geometry::{CellSliceResult, slice_hex, slice_tet};
```

and replace with:

```rust
pub use slice_geometry::{CellSliceResult, slice_hex, slice_prism, slice_pyramid, slice_tet};
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test -p polyscope-structures slice_prism slice_pyramid -- --nocapture`
Expected: PASS.

Run the full slice geometry suite:

Run: `cargo test -p polyscope-structures slice_geometry -- --nocapture`
Expected: All PASS.

- [ ] **Step 6: Run clippy**

Run: `cargo clippy --workspace -- -D warnings`
Expected: Zero warnings.

- [ ] **Step 7: Commit**

```bash
git add crates/polyscope-structures/src/volume_mesh/slice_geometry.rs crates/polyscope-structures/src/volume_mesh/mod.rs
git commit -m "feat(volume_mesh): slice_prism / slice_pyramid via tet decomposition"
```

---

### Task 6: Public constructors and registration functions

**Files:**
- Modify: `crates/polyscope-structures/src/volume_mesh/mod.rs` (add `new_prism_mesh`, `new_pyramid_mesh`)
- Modify: `crates/polyscope/src/volume_mesh.rs` (add `register_prism_mesh`, `register_pyramid_mesh`)
- Modify: `crates/polyscope/src/lib.rs` if `register_*` functions are explicitly re-exported (verify by grep first)

- [ ] **Step 1: Write the failing test**

In `crates/polyscope-structures/src/volume_mesh/mod.rs`, append:

```rust
#[test]
fn test_new_prism_mesh_constructor() {
    let verts = vec![
        Vec3::ZERO, Vec3::X, Vec3::Y,
        Vec3::Z, Vec3::X + Vec3::Z, Vec3::Y + Vec3::Z,
    ];
    let prisms = vec![[0u32, 1, 2, 3, 4, 5]];
    let mesh = VolumeMesh::new_prism_mesh("p", verts, prisms);
    assert_eq!(mesh.num_cells(), 1);
    assert_eq!(mesh.cell_type(0), VolumeCellType::Prism);
}

#[test]
fn test_new_pyramid_mesh_constructor() {
    let verts = vec![
        Vec3::ZERO, Vec3::X, Vec3::X + Vec3::Y, Vec3::Y,
        Vec3::splat(0.5) + Vec3::Z,
    ];
    let pyramids = vec![[0u32, 1, 2, 3, 4]];
    let mesh = VolumeMesh::new_pyramid_mesh("py", verts, pyramids);
    assert_eq!(mesh.num_cells(), 1);
    assert_eq!(mesh.cell_type(0), VolumeCellType::Pyramid);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p polyscope-structures test_new_prism_mesh_constructor test_new_pyramid_mesh_constructor -- --nocapture`
Expected: FAIL — methods not defined.

- [ ] **Step 3: Add `new_prism_mesh` and `new_pyramid_mesh`**

In `crates/polyscope-structures/src/volume_mesh/mod.rs`, after `new_hex_mesh` (after line 175), insert:

```rust
    /// Creates a triangular-prism (wedge) mesh.
    ///
    /// Each prism has 6 vertices: slots 0..2 form the bottom triangle and
    /// slots 3..5 form the top triangle (slot `i+3` should be the vertex
    /// above slot `i`). Cells are stored as 8-index arrays with the last two
    /// slots set to `u32::MAX` for sentinel detection.
    pub fn new_prism_mesh(
        name: impl Into<String>,
        vertices: Vec<Vec3>,
        prisms: Vec<[u32; 6]>,
    ) -> Self {
        let cells: Vec<[u32; 8]> = prisms
            .into_iter()
            .map(|p| [p[0], p[1], p[2], p[3], p[4], p[5], u32::MAX, u32::MAX])
            .collect();
        Self::new(name, vertices, cells)
    }

    /// Creates a square-pyramid mesh.
    ///
    /// Each pyramid has 5 vertices: slots 0..3 form the base quad (in CCW
    /// order viewed from outside the cell) and slot 4 is the apex. Cells are
    /// stored as 8-index arrays with the last three slots set to `u32::MAX`.
    pub fn new_pyramid_mesh(
        name: impl Into<String>,
        vertices: Vec<Vec3>,
        pyramids: Vec<[u32; 5]>,
    ) -> Self {
        let cells: Vec<[u32; 8]> = pyramids
            .into_iter()
            .map(|p| [p[0], p[1], p[2], p[3], p[4], u32::MAX, u32::MAX, u32::MAX])
            .collect();
        Self::new(name, vertices, cells)
    }
```

- [ ] **Step 4: Add the registration wrappers**

In `crates/polyscope/src/volume_mesh.rs`, after `register_hex_mesh` (after line 69), insert:

```rust
/// Registers a triangular-prism (wedge) mesh with polyscope.
///
/// Each entry in `prisms` is 6 vertex indices: slots 0..2 = bottom triangle,
/// slots 3..5 = top triangle (with slot `i+3` directly above slot `i`).
pub fn register_prism_mesh(
    name: impl Into<String>,
    vertices: Vec<Vec3>,
    prisms: Vec<[u32; 6]>,
) -> VolumeMeshHandle {
    let name = name.into();
    let mesh = VolumeMesh::new_prism_mesh(name.clone(), vertices, prisms);

    with_context_mut(|ctx| {
        ctx.registry
            .register(Box::new(mesh))
            .expect("failed to register prism mesh");
        ctx.update_extents();
    });

    VolumeMeshHandle { name }
}

/// Registers a square-pyramid mesh with polyscope.
///
/// Each entry in `pyramids` is 5 vertex indices: slots 0..3 = base quad (CCW
/// from outside), slot 4 = apex.
pub fn register_pyramid_mesh(
    name: impl Into<String>,
    vertices: Vec<Vec3>,
    pyramids: Vec<[u32; 5]>,
) -> VolumeMeshHandle {
    let name = name.into();
    let mesh = VolumeMesh::new_pyramid_mesh(name.clone(), vertices, pyramids);

    with_context_mut(|ctx| {
        ctx.registry
            .register(Box::new(mesh))
            .expect("failed to register pyramid mesh");
        ctx.update_extents();
    });

    VolumeMeshHandle { name }
}
```

- [ ] **Step 5: Check if re-exports are needed**

Run: `grep -n "register_tet_mesh\|register_hex_mesh" crates/polyscope/src/lib.rs`

If the existing `register_tet_mesh` and `register_hex_mesh` appear in an explicit `pub use` list, add `register_prism_mesh` and `register_pyramid_mesh` to the same list. If they are auto-exported via `pub use crate::volume_mesh::*;` or similar wildcard, no change is needed.

If a `pub use` exists like:
```rust
pub use crate::volume_mesh::{register_hex_mesh, register_tet_mesh, register_volume_mesh, VolumeMeshHandle, ...};
```
extend it to:
```rust
pub use crate::volume_mesh::{register_hex_mesh, register_prism_mesh, register_pyramid_mesh, register_tet_mesh, register_volume_mesh, VolumeMeshHandle, ...};
```

- [ ] **Step 6: Run all tests**

Run: `cargo test --workspace -- --nocapture`
Expected: All PASS.

- [ ] **Step 7: Run clippy**

Run: `cargo clippy --workspace -- -D warnings`
Expected: Zero warnings.

- [ ] **Step 8: Commit**

```bash
git add crates/polyscope-structures/src/volume_mesh/mod.rs crates/polyscope/src/volume_mesh.rs crates/polyscope/src/lib.rs
git commit -m "feat(volume_mesh): public register_prism_mesh / register_pyramid_mesh"
```

---

### Task 7: Visual demo example

**Files:**
- Create: `examples/volume_mesh_mixed_cells_demo.rs`

- [ ] **Step 1: Write the example**

Look at `examples/volume_mesh_demo.rs` (13K, has the existing layout) to confirm the import style and structure. Then create `examples/volume_mesh_mixed_cells_demo.rs`:

```rust
//! Demonstrates all four volume-mesh cell types side by side: tet, hex,
//! prism (wedge), and pyramid. Each cell is rendered as its own mesh so
//! the structure list shows them separately.

use polyscope_rs::*;

fn main() -> Result<()> {
    init()?;

    // ----- Tet -----
    let tet_verts = vec![
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(1.0, 0.0, 0.0),
        Vec3::new(0.5, 1.0, 0.0),
        Vec3::new(0.5, 0.5, 1.0),
    ];
    let tet = register_tet_mesh("tet", tet_verts, vec![[0, 1, 2, 3]]);
    tet.add_vertex_scalar_quantity("z", vec![0.0, 0.0, 0.0, 1.0]);

    // ----- Hex (translated +3 in x) -----
    let dx = Vec3::new(3.0, 0.0, 0.0);
    let hex_verts = vec![
        Vec3::new(0.0, 0.0, 0.0) + dx,
        Vec3::new(1.0, 0.0, 0.0) + dx,
        Vec3::new(1.0, 1.0, 0.0) + dx,
        Vec3::new(0.0, 1.0, 0.0) + dx,
        Vec3::new(0.0, 0.0, 1.0) + dx,
        Vec3::new(1.0, 0.0, 1.0) + dx,
        Vec3::new(1.0, 1.0, 1.0) + dx,
        Vec3::new(0.0, 1.0, 1.0) + dx,
    ];
    let hex = register_hex_mesh("hex", hex_verts, vec![[0, 1, 2, 3, 4, 5, 6, 7]]);
    hex.add_vertex_scalar_quantity("z", vec![0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 1.0]);

    // ----- Prism (translated +6 in x) -----
    let dx = Vec3::new(6.0, 0.0, 0.0);
    let prism_verts = vec![
        Vec3::new(0.0, 0.0, 0.0) + dx,
        Vec3::new(1.0, 0.0, 0.0) + dx,
        Vec3::new(0.5, 1.0, 0.0) + dx,
        Vec3::new(0.0, 0.0, 1.0) + dx,
        Vec3::new(1.0, 0.0, 1.0) + dx,
        Vec3::new(0.5, 1.0, 1.0) + dx,
    ];
    let prism = register_prism_mesh("prism", prism_verts, vec![[0, 1, 2, 3, 4, 5]]);
    prism.add_vertex_scalar_quantity("z", vec![0.0, 0.0, 0.0, 1.0, 1.0, 1.0]);

    // ----- Pyramid (translated +9 in x) -----
    let dx = Vec3::new(9.0, 0.0, 0.0);
    let pyr_verts = vec![
        Vec3::new(0.0, 0.0, 0.0) + dx,
        Vec3::new(1.0, 0.0, 0.0) + dx,
        Vec3::new(1.0, 1.0, 0.0) + dx,
        Vec3::new(0.0, 1.0, 0.0) + dx,
        Vec3::new(0.5, 0.5, 1.0) + dx,
    ];
    let pyr = register_pyramid_mesh("pyramid", pyr_verts, vec![[0, 1, 2, 3, 4]]);
    pyr.add_vertex_scalar_quantity("z", vec![0.0, 0.0, 0.0, 0.0, 1.0]);

    show();
    Ok(())
}
```

- [ ] **Step 2: Build the example to verify it compiles**

Run: `cargo build --example volume_mesh_mixed_cells_demo`
Expected: Builds clean.

- [ ] **Step 3: Run clippy on the example**

Run: `cargo clippy --example volume_mesh_mixed_cells_demo -- -D warnings`
Expected: Zero warnings.

- [ ] **Step 4: Run the demo manually and verify visually**

Run: `cargo run --release --example volume_mesh_mixed_cells_demo`

Visual checks:
- Four distinct shapes appear in a row along the x axis.
- The structure list on the right panel shows four entries: `tet`, `hex`, `prism`, `pyramid`.
- Each has a "z" scalar quantity selectable in its UI.
- Toggling visibility hides only that one cell.
- Hovering / clicking each cell highlights it (picking should work — confirms the per-triangle cell-index mapping is correct).

If the visual check passes, proceed. If anything looks wrong (especially mismatched faces, gaps, or wrong shapes), debug before committing — the most likely culprit is a wrong vertex order in `PRISM_FACES` / `PYRAMID_FACES` in `cell_data.rs`.

- [ ] **Step 5: Commit**

```bash
git add examples/volume_mesh_mixed_cells_demo.rs
git commit -m "docs(examples): add mixed-cell-type volume mesh demo"
```

---

### Task 8: Update documentation

**Files:**
- Modify: `docs/feature-status.md`
- Modify: `CHANGELOG.md`

- [ ] **Step 1: Update `feature-status.md`**

In `docs/feature-status.md`, find the Volume Mesh row in the Structures table (around line 12):

```markdown
| Volume Mesh | Full | Full | Tet/hex, interior face detection, slice capping |
```

Update it to:

```markdown
| Volume Mesh | Full | Full | Tet/hex/prism/pyramid, interior face detection, slice capping |
```

Find the `## Completed Features` section (around line 60) and add a new entry under the existing upstream-port lines (after the entry for inverse-view interpolation, line 79):

```markdown
- [x] Volume Mesh prism + pyramid cell support (upstream PR #353, commit dcbaedb)
```

Find the `### Upstream Ports (Medium-Term)` subsection under Planned Work — no removal needed, the prism/pyramid item was not previously listed there.

- [ ] **Step 2: Update `CHANGELOG.md`**

In `CHANGELOG.md`, add a new entry at the top of the file (above `## [0.5.9]`):

```markdown
## [Unreleased]

### Added
- Volume Mesh now supports prism (wedge) and pyramid cell types in addition to
  tetrahedra and hexahedra. New constructors: `register_prism_mesh`,
  `register_pyramid_mesh`, `VolumeMesh::new_prism_mesh`,
  `VolumeMesh::new_pyramid_mesh`. Mixed-cell meshes are supported by using the
  8-slot cell array with sentinel `u32::MAX` indices in unused slots
  (matches upstream Polyscope PR #353).
- `slice_prism` and `slice_pyramid` helpers in
  `polyscope_structures::volume_mesh::slice_geometry`.
- Example `volume_mesh_mixed_cells_demo` showing all four cell types.

### Changed
- `VolumeMesh::cell_type` now classifies cells by sentinel count (0/2/3/4 →
  Hex/Prism/Pyramid/Tet) instead of by `cell[4] == u32::MAX`. Mixed meshes
  produced by external pipelines must place sentinels in the correct trailing
  slots; tet meshes built with `new_tet_mesh` continue to work unchanged.
```

- [ ] **Step 3: Verify the docs build / render correctly**

Run: `head -30 CHANGELOG.md`
Expected: New `## [Unreleased]` section sits cleanly above `## [0.5.9]`.

- [ ] **Step 4: Commit**

```bash
git add docs/feature-status.md CHANGELOG.md
git commit -m "docs: prism/pyramid cell support (upstream PR #353)"
```

---

## Self-Review

### Spec coverage

Upstream PR #353 adds:
1. PRISM and PYRAMID enum variants — Task 1 ✓
2. Sentinel-count cell-type detection — Task 2 ✓
3. Per-cell-type face stencils + face polygons + real-edge stencils — Task 1 (stencils + polygons); real-edge stencils are only used for wireframe rendering, which polyscope-rs does not currently expose for volume meshes; not required for parity at the geometry level.
4. `decomposePrism` / `decomposePyramid` — Task 3 ✓
5. Slice integration through `computeTets` — Tasks 3 + 5 ✓
6. Constructors taking mixed cells — Task 6 (`new_prism_mesh` / `new_pyramid_mesh` plus pre-existing `register_volume_mesh` for mixed) ✓
7. UI integration / menu / picking — already cell-type-agnostic in polyscope-rs because dispatch goes through `cell_type()`; verified visually in Task 7.

Edge cases left intentionally out of scope:
- Wireframe edges along "real" cell edges (would require porting the `realEdgeStencil`). Tracked separately if/when polyscope-rs adds volume-mesh edge rendering.
- Categorical-data isosurface on prism/pyramid (the polyscope-rs port does isosurface on volume **grids**, not on volume **meshes** with level-set quantities, so the upstream `activeLevelSetQuantity` path is not exercised the same way).

### Placeholder scan

No "TBD" / "implement later" / "similar to Task N" / "add appropriate handling" — every step has a complete code block or an exact command.

### Type consistency

- `VolumeCellType` variants `Tet`, `Hex`, `Prism`, `Pyramid` used consistently across Tasks 1–8.
- `decompose_prism` returns `[[u32; 4]; 3]` and `decompose_pyramid` returns `[[u32; 4]; 2]`; the dispatcher `decompose_cell_to_tets` wraps both in `Vec<[u32; 4]>`. Consistent.
- `slice_prism` takes `[Vec3; 6]`, `slice_pyramid` takes `[Vec3; 5]` (matching upstream and the new constructors).
- `register_prism_mesh(prisms: Vec<[u32; 6]>)` and `register_pyramid_mesh(pyramids: Vec<[u32; 5]>)` mirror the inner `new_prism_mesh` / `new_pyramid_mesh` signatures.
- `canonical_face_key` signature: `fn(&[u32; 8], &[usize]) -> [u32; 4]`. Same call site uses `face.polygon` (`&'static [usize]`) — type-compatible.

---

## Execution Handoff

**Plan complete and saved to `docs/plans/2026-05-19-volume-mesh-prism-pyramid.md`. Two execution options:**

**1. Subagent-Driven (recommended)** — I dispatch a fresh subagent per task, review between tasks, fast iteration.

**2. Inline Execution** — Execute tasks in this session using executing-plans, batch execution with checkpoints.

**Which approach?**
