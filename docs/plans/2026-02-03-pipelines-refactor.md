# Pipelines Module Refactoring Plan

**Date:** 2026-02-03  
**Goal:** Split `engine/pipelines.rs` (1,863 lines) into a `pipelines/` directory with 3 module files to improve maintainability and stay under the 2,000-line limit.

## Problem Statement

Per CLAUDE.md guidelines, no single source file should exceed 2,000 lines. The `crates/polyscope-render/src/engine/pipelines.rs` file is at 1,863 lines and approaching this limit. The file contains all GPU pipeline creation code for various structure types and effects.

## Current File Structure

```
crates/polyscope-render/src/engine/
├── mod.rs              (1,215 lines)
├── pipelines.rs        (1,863 lines) ⚠️
├── pick.rs             (832 lines)
├── postprocessing.rs   (486 lines)
├── rendering.rs        (579 lines)
└── textures.rs         (202 lines)
```

## Current Content of pipelines.rs

| Lines | Function | Description |
|-------|----------|-------------|
| 9-126 | `init_point_pipeline()` | Point sphere impostor rendering |
| 129-133 | `point_bind_group_layout()` | Accessor |
| 136-253 | `init_vector_pipeline()` | Vector arrow rendering |
| 256-260 | `vector_bind_group_layout()` | Accessor |
| 263-267 | `mesh_bind_group_layout()` | Accessor |
| 270-274 | `simple_mesh_bind_group_layout()` | Accessor |
| 277-281 | `gridcube_bind_group_layout()` | Accessor |
| 284-495 | `create_mesh_pipeline()` | Surface mesh + depth/normal pipelines |
| 498-502 | `curve_network_edge_bind_group_layout()` | Accessor |
| 505-644 | `create_curve_network_edge_pipeline()` | Line rendering |
| 647-867 | `create_curve_network_tube_pipelines()` | Compute + render pipelines |
| 870-888 | `curve_network_tube_*()` | 4 accessor methods |
| 891-996 | `create_shadow_pipeline()` | Shadow map depth pass |
| 999-1006 | `shadow_*()` | 2 accessor methods |
| 1009-1079 | `create_ground_stencil_pipeline()` | Reflection stencil mask |
| 1082-1262 | `create_reflected_mesh_pipeline()` | Reflected surface mesh |
| 1264-1408 | `create_reflected_point_cloud_pipeline()` | Reflected point cloud |
| 1410-1575 | `create_reflected_curve_network_pipeline()` | Reflected curve network |
| 1577-1704 | `create_simple_mesh_pipeline()` | Isosurface rendering |
| 1706-1862 | `create_gridcube_pipeline()` | Volume grid visualization |

## Proposed Split (Coarse-Grained)

Split by logical grouping into 3 files:

```
crates/polyscope-render/src/engine/
├── pipelines/
│   ├── mod.rs              (~10 lines)   - Module declarations only
│   ├── structure.rs        (~810 lines)  - Point, vector, mesh, curve pipelines
│   ├── effects.rs          (~700 lines)  - Shadow, reflection pipelines
│   └── volume.rs           (~175 lines)  - Gridcube pipeline
└── [pipelines.rs deleted]
```

### `pipelines/mod.rs` (~10 lines)

Module declarations only:
```rust
//! Pipeline creation and accessor functions for the render engine.

mod effects;
mod structure;
mod volume;
```

### `pipelines/structure.rs` (~810 lines)

Pipelines for core visualization structures:

| Function | Est. Lines | Description |
|----------|------------|-------------|
| `init_point_pipeline()` | ~120 | Point sphere rendering |
| `point_bind_group_layout()` | ~5 | Accessor |
| `init_vector_pipeline()` | ~120 | Vector arrow rendering |
| `vector_bind_group_layout()` | ~5 | Accessor |
| `create_mesh_pipeline()` | ~210 | Surface mesh + depth/normal |
| `mesh_bind_group_layout()` | ~5 | Accessor |
| `create_simple_mesh_pipeline()` | ~130 | Isosurface mesh |
| `simple_mesh_bind_group_layout()` | ~5 | Accessor |
| `create_curve_network_edge_pipeline()` | ~140 | Line rendering |
| `curve_network_edge_bind_group_layout()` | ~5 | Accessor |
| `create_curve_network_tube_pipelines()` | ~220 | Compute + render |
| `curve_network_tube_*()` | ~20 | 4 accessors |

### `pipelines/effects.rs` (~700 lines)

Pipelines for visual effects:

| Function | Est. Lines | Description |
|----------|------------|-------------|
| `create_shadow_pipeline()` | ~110 | Shadow map depth pass |
| `shadow_pipeline()` | ~5 | Accessor |
| `shadow_bind_group_layout()` | ~5 | Accessor |
| `create_ground_stencil_pipeline()` | ~70 | Reflection stencil mask |
| `create_reflected_mesh_pipeline()` | ~180 | Reflected surface mesh |
| `create_reflected_point_cloud_pipeline()` | ~145 | Reflected point cloud |
| `create_reflected_curve_network_pipeline()` | ~165 | Reflected curve network |

### `pipelines/volume.rs` (~175 lines)

Pipelines for volume visualization:

| Function | Est. Lines | Description |
|----------|------------|-------------|
| `create_gridcube_pipeline()` | ~160 | Volume grid cube rendering |
| `gridcube_bind_group_layout()` | ~5 | Accessor |

## Implementation Steps

1. **Create `pipelines/` directory**
2. **Create `pipelines/mod.rs`** with module declarations
3. **Create `pipelines/structure.rs`**:
   - Add file header and imports
   - Move point pipeline + accessor
   - Move vector pipeline + accessor
   - Move mesh pipeline + accessor
   - Move simple mesh pipeline + accessor
   - Move curve network edge pipeline + accessor
   - Move curve network tube pipelines + accessors
4. **Create `pipelines/effects.rs`**:
   - Add file header and imports
   - Move shadow pipeline + accessors
   - Move ground stencil pipeline
   - Move reflected mesh pipeline
   - Move reflected point cloud pipeline
   - Move reflected curve network pipeline
5. **Create `pipelines/volume.rs`**:
   - Add file header and imports
   - Move gridcube pipeline + accessor
6. **Delete old `pipelines.rs`**
7. **Verify** with `cargo build`, `cargo test`, `cargo clippy`

## Required Imports

All three submodules need:
```rust
use std::num::NonZeroU64;
use super::super::RenderEngine;
```

## Dependencies / Callers

Files that call pipeline methods (no changes needed - they use `RenderEngine` methods):

- `engine/mod.rs` - calls `engine.init_point_pipeline()` etc. in `new_windowed()` and `new_headless()`
- `engine/postprocessing.rs` - calls `self.create_mesh_pipeline()` in depth peel initialization
- `polyscope/src/app/render.rs` - calls `engine.*_bind_group_layout()` accessors
- `polyscope/src/app/render_init.rs` - calls `engine.*_bind_group_layout()` accessors

## Risks & Mitigations

| Risk | Mitigation |
|------|------------|
| Method visibility changes | Keep exact same visibility (`pub`, `pub(crate)`) |
| Import paths broken | `super::super::RenderEngine` correctly references parent module |
| Tests fail | No tests in pipelines.rs; tests are integration tests in other crates |

## Verification

```bash
cargo build                    # Must succeed
cargo test                     # All tests must pass
cargo clippy                   # Zero warnings required
wc -l crates/polyscope-render/src/engine/pipelines/*.rs  # Verify line counts
```

## Success Criteria

- [x] `cargo build` succeeds
- [x] `cargo test` passes all tests
- [x] `cargo clippy` shows zero warnings
- [x] `structure.rs` ≤ 850 lines
- [x] `effects.rs` ≤ 750 lines
- [x] `volume.rs` ≤ 200 lines
- [x] No public API changes
