# Surface Mesh Module Refactoring Plan

**Date:** 2026-02-02  
**Goal:** Split `surface_mesh/mod.rs` (1,956 lines) into smaller, focused modules to stay under the 2,000-line limit and improve maintainability.

## Problem Statement

Per CLAUDE.md guidelines, no single source file should exceed 2,000 lines. The `crates/polyscope-structures/src/surface_mesh/mod.rs` file is at 1,956 lines and approaching this limit. Continued development will push it over.

## Current File Structure

```
crates/polyscope-structures/src/surface_mesh/
├── mod.rs                           (1,956 lines) ⚠️
├── intrinsic_vector_quantity.rs     (640 lines)
├── one_form_quantity.rs             (~400 lines)
├── parameterization_quantity.rs     (~350 lines)
└── quantities.rs                    (879 lines)
```

## Proposed Split

### New Module: `geometry.rs` (~220 lines)

Extract geometry computation methods that derive mesh topology data from raw vertices/faces:

| Method | Lines | Description |
|--------|-------|-------------|
| `recompute()` | ~15 | Orchestrates all computation |
| `compute_triangulation()` | ~20 | Fan triangulation for polygons |
| `compute_face_normals()` | ~18 | Cross product of edges |
| `compute_vertex_normals()` | ~35 | Area-weighted face normals |
| `compute_corner_normals()` | ~25 | Per-corner for shading |
| `compute_edge_is_real()` | ~30 | Wireframe edge classification |
| `compute_edges()` | ~20 | Unique edge extraction |
| `compute_face_tangent_basis()` | ~25 | For intrinsic vectors |
| `compute_vertex_tangent_basis()` | ~55 | For intrinsic vectors |

**Implementation**: Separate `impl SurfaceMesh` block in new file. Rust allows multiple impl blocks across files within the same module.

### New Module: `quantity_methods.rs` (~480 lines)

Extract all quantity add/get methods for each quantity type:

**Add methods** (~180 lines):
- `add_vertex_scalar_quantity()`
- `add_face_scalar_quantity()`
- `add_vertex_color_quantity()`
- `add_vertex_color_quantity_with_alpha()`
- `add_face_color_quantity()`
- `add_face_color_quantity_with_alpha()`
- `add_vertex_vector_quantity()`
- `add_face_vector_quantity()`
- `add_vertex_parameterization_quantity()`
- `add_corner_parameterization_quantity()`
- `add_vertex_intrinsic_vector_quantity()`
- `add_vertex_intrinsic_vector_quantity_auto()`
- `add_face_intrinsic_vector_quantity()`
- `add_face_intrinsic_vector_quantity_auto()`
- `add_one_form_quantity()`

**Active quantity accessors** (~280 lines):
- `active_vertex_scalar_quantity()`
- `active_face_scalar_quantity()`
- `active_vertex_color_quantity()`
- `active_face_color_quantity()`
- `active_vertex_vector_quantity()`
- `active_vertex_vector_quantity_mut()`
- `active_face_vector_quantity()`
- `active_face_vector_quantity_mut()`
- `active_vertex_parameterization_quantity()`
- `active_corner_parameterization_quantity()`
- `active_vertex_intrinsic_vector_quantity()`
- `active_vertex_intrinsic_vector_quantity_mut()`
- `active_face_intrinsic_vector_quantity()`
- `active_face_intrinsic_vector_quantity_mut()`
- `active_one_form_quantity()`
- `active_one_form_quantity_mut()`

**Helper methods** (~20 lines):
- `face_centroids()`

### Updated `mod.rs` (~1,250 lines)

Retains:
- Module imports and re-exports
- `ShadeStyle` and `BackfacePolicy` enums
- `SurfaceMesh` struct definition
- Constructor methods (`new`, `from_triangles`)
- Simple getters/setters (lines 146-328)
- `build_egui_ui()` method (lines 507-619)
- GPU resource methods (lines 1180-1421)
- `impl Structure` trait (lines 1423-1537)
- `impl HasQuantities` trait (lines 1539-1563)
- Unit tests (lines 1565-1956)

Adds at top:
```rust
mod geometry;
mod quantity_methods;
```

## Final File Structure

```
crates/polyscope-structures/src/surface_mesh/
├── mod.rs                           (~1,250 lines) ✅
├── geometry.rs                      (~220 lines)   NEW
├── quantity_methods.rs              (~480 lines)   NEW
├── intrinsic_vector_quantity.rs     (640 lines)
├── one_form_quantity.rs             (~400 lines)
├── parameterization_quantity.rs     (~350 lines)
└── quantities.rs                    (879 lines)
```

## Implementation Steps

1. **Create `geometry.rs`**:
   - Add file header with module documentation
   - Add necessary imports (`glam::Vec3`, `std::collections::HashSet`)
   - Move geometry computation methods into `impl SurfaceMesh` block
   - Change `fn` to `pub(super) fn` for internal methods called by `mod.rs`
   - Keep `pub fn` for methods part of public API

2. **Create `quantity_methods.rs`**:
   - Add file header with module documentation
   - Add necessary imports (quantity types, `QuantityKind`, `HasQuantities`)
   - Move quantity methods into `impl SurfaceMesh` block
   - All methods remain `pub fn` (public API)

3. **Update `mod.rs`**:
   - Add `mod geometry;` and `mod quantity_methods;` after existing mod declarations
   - Remove the extracted method implementations
   - Keep all struct fields, trait impls, and tests

4. **Verification**:
   ```bash
   cargo build
   cargo test -p polyscope-structures
   cargo clippy
   cargo fmt
   ```

## Dependencies

`geometry.rs` requires:
- `glam::Vec3`
- `std::collections::HashSet`
- Access to `SurfaceMesh` fields (same crate)
- `ShadeStyle` enum

`quantity_methods.rs` requires:
- `glam::{Vec2, Vec3, Vec4}`
- `polyscope_core::quantity::QuantityKind`
- `polyscope_core::structure::HasQuantities`
- All quantity types from sibling modules
- Access to `SurfaceMesh` fields

## Risks & Mitigations

| Risk | Mitigation |
|------|------------|
| Method visibility changes break external API | All `pub fn` methods stay `pub fn`; only internal helpers become `pub(super)` |
| Circular imports | Both new modules only reference `SurfaceMesh` from parent; no cross-references |
| Test failures | Tests remain in `mod.rs`, have access to all impl blocks |

## Success Criteria

- [x] `cargo build` succeeds
- [x] `cargo test -p polyscope-structures` passes all existing tests (77 tests)
- [x] `cargo clippy` shows zero warnings
- [x] `mod.rs` is under 1,300 lines (now 1,224 lines)
- [x] No public API changes (all existing methods accessible)
