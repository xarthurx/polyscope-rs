# render.rs Split + Dedup Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Split the 3,459-line `render.rs` into 5 focused modules, deduplicating 3x scene drawing code and 2x GPU init code.

**Architecture:** Extract shared functions for GPU initialization, buffer updates, and scene draw commands into dedicated modules. The main `render.rs` becomes a slim orchestrator. Screenshot and headless paths call shared helpers instead of duplicating draw logic. All modules use `impl App` or free functions taking `&RenderEngine`.

**Tech Stack:** Rust, wgpu, egui, polyscope-rs workspace

---

## Pre-flight

Before starting, verify the current state compiles and tests pass:

```bash
cargo build 2>&1 | tail -5
cargo test 2>&1 | tail -10
cargo clippy 2>&1 | tail -5
```

All must be clean. If not, fix first.

---

## Task 1: Create `render_scene.rs` — shared draw command helpers

This is the highest-value extraction: scene drawing code is duplicated 3 times.

**Files:**
- Create: `crates/polyscope/src/app/render_scene.rs`
- Modify: `crates/polyscope/src/app/mod.rs` (add `mod render_scene;`)

**Step 1: Create `render_scene.rs` with all shared draw functions**

Extract the following draw functions. Each takes a `&mut wgpu::RenderPass` and `&RenderEngine` and draws all visible structures of a given type. The pattern is identical across all 3 call sites (render, capture_screenshot, capture_screenshot_headless).

```rust
// crates/polyscope/src/app/render_scene.rs
//! Shared scene drawing commands used by windowed, screenshot, and headless render paths.

use super::{PointCloud, SurfaceMesh, CurveNetwork, CameraView, VolumeGrid, VolumeMesh};
use polyscope_structures::volume_grid::{VolumeGridNodeScalarQuantity, VolumeGridCellScalarQuantity, VolumeGridVizMode};
use polyscope_core::structure::HasQuantities;
use polyscope_core::quantity::Quantity;
use polyscope_render::RenderEngine;

/// Draw all visible point clouds.
pub(super) fn draw_point_clouds<'a>(
    render_pass: &mut wgpu::RenderPass<'a>,
    engine: &'a RenderEngine,
) {
    // Copy from render.rs lines 1884-1904 (the point cloud draw block)
    // Use engine.point_pipeline, iterate registry, draw each PC
}

/// Draw all visible vector quantities (PointCloud + SurfaceMesh vectors).
pub(super) fn draw_vector_quantities<'a>(
    render_pass: &mut wgpu::RenderPass<'a>,
    engine: &'a RenderEngine,
) {
    // Copy from render.rs lines 1907-1970
}

/// Draw curve network edges (line mode), camera views, and volume grid wireframes.
pub(super) fn draw_curve_networks_and_lines<'a>(
    render_pass: &mut wgpu::RenderPass<'a>,
    engine: &'a RenderEngine,
) {
    // Copy from render.rs lines 1975-2018
}

/// Draw curve network tubes (tube mode).
pub(super) fn draw_curve_network_tubes<'a>(
    render_pass: &mut wgpu::RenderPass<'a>,
    engine: &'a RenderEngine,
) {
    // Copy from render.rs lines 2020-2051
}

/// Draw curve network node spheres (tube mode joint fill).
pub(super) fn draw_curve_network_nodes<'a>(
    render_pass: &mut wgpu::RenderPass<'a>,
    engine: &'a RenderEngine,
) {
    // Copy from render.rs lines 2053-2080
}

/// Draw surface meshes and volume meshes (indexed draw, for MRT pass).
/// Used in simple/none transparency mode.
pub(super) fn draw_meshes_simple<'a>(
    render_pass: &mut wgpu::RenderPass<'a>,
    engine: &'a RenderEngine,
) {
    // Copy from render.rs lines 2214-2266 (the else branch - simple mode)
}

/// Draw volume grid isosurfaces (simple mesh pipeline).
pub(super) fn draw_volume_grid_isosurfaces<'a>(
    render_pass: &mut wgpu::RenderPass<'a>,
    engine: &'a RenderEngine,
) {
    // Copy from render.rs lines 2269-2297
}

/// Draw volume grid gridcubes (gridcube pipeline).
pub(super) fn draw_volume_grid_gridcubes<'a>(
    render_pass: &mut wgpu::RenderPass<'a>,
    engine: &'a RenderEngine,
) {
    // Copy from render.rs lines 2299-2334
}
```

**Step 2: Add module declaration in `mod.rs`**

Add `mod render_scene;` to `crates/polyscope/src/app/mod.rs` alongside the existing module declarations.

**Step 3: Build and verify**

```bash
cargo build 2>&1 | tail -5
```

Expected: Compiles (new module exists but isn't called yet).

**Step 4: Commit**

```bash
git add -A && git commit -m "refactor: extract shared draw commands into render_scene.rs"
```

---

## Task 2: Wire `render_scene.rs` into the main `render()` function

**Files:**
- Modify: `crates/polyscope/src/app/render.rs` (replace inline draw blocks with calls to render_scene)

**Step 1: Replace main render pass draw blocks**

In `render()`, replace the inline draw blocks (lines ~1884-2080) with calls to the new helpers:

```rust
// Was: inline point cloud drawing (lines 1884-1904)
render_scene::draw_point_clouds(&mut render_pass, engine);

// Was: inline vector quantity drawing (lines 1907-1970)
render_scene::draw_vector_quantities(&mut render_pass, engine);

// Was: inline curve network line drawing (lines 1975-2018)
render_scene::draw_curve_networks_and_lines(&mut render_pass, engine);

// Was: inline curve network tube drawing (lines 2020-2051)
render_scene::draw_curve_network_tubes(&mut render_pass, engine);

// Was: inline curve network node drawing (lines 2053-2080)
render_scene::draw_curve_network_nodes(&mut render_pass, engine);
```

For the MRT surface mesh pass (lines ~2083-2335), replace the simple-mode branch with:
```rust
render_scene::draw_meshes_simple(&mut render_pass, engine);
```

And after the mesh pipeline setup, replace isosurface/gridcube blocks:
```rust
render_scene::draw_volume_grid_isosurfaces(&mut render_pass, engine);
render_scene::draw_volume_grid_gridcubes(&mut render_pass, engine);
```

**NOTE:** The depth-peel mode branch (lines 2134-2205) has slightly different logic (depth-only for SurfaceMesh, full for VolumeMesh). Keep that inline or add a separate `draw_meshes_depth_peel_prepass()` helper.

**Step 2: Build and test**

```bash
cargo build 2>&1 | tail -5
cargo test 2>&1 | tail -10
cargo clippy 2>&1 | tail -5
```

All must pass.

**Step 3: Commit**

```bash
git add -A && git commit -m "refactor: use render_scene helpers in main render pass"
```

---

## Task 3: Wire `render_scene.rs` into screenshot paths (dedup)

**Files:**
- Modify: `crates/polyscope/src/app/render.rs` — `capture_screenshot()` and `capture_screenshot_headless()`

**Step 1: Replace draw blocks in `capture_screenshot()`**

Replace lines ~2581-2745 (all the inline draw blocks in the screenshot render pass) with calls to the shared helpers:

```rust
render_scene::draw_point_clouds(&mut render_pass, engine);
render_scene::draw_vector_quantities(&mut render_pass, engine);
render_scene::draw_meshes_simple(&mut render_pass, engine);
render_scene::draw_curve_networks_and_lines(&mut render_pass, engine);
```

**Step 2: Replace draw blocks in `capture_screenshot_headless()`**

Replace lines ~3167-3305 with the same shared helper calls. Also replace the separate MRT mesh pass (lines ~3314-3406):

```rust
render_scene::draw_meshes_simple(&mut render_pass, engine);
```

**Step 3: Build and test**

```bash
cargo build 2>&1 | tail -5
cargo test 2>&1 | tail -10
cargo clippy 2>&1 | tail -5
```

**Step 4: Commit**

```bash
git add -A && git commit -m "refactor: dedup screenshot/headless rendering via render_scene helpers"
```

---

## Task 4: Create `render_init.rs` — shared GPU initialization

**Files:**
- Create: `crates/polyscope/src/app/render_init.rs`
- Modify: `crates/polyscope/src/app/mod.rs` (add `mod render_init;`)

**Step 1: Create `render_init.rs` with shared init functions**

```rust
// crates/polyscope/src/app/render_init.rs
//! Shared GPU resource initialization and buffer update logic.

use super::{App, PointCloud, SurfaceMesh, CurveNetwork, CameraView, VolumeGrid, VolumeMesh, Vec3, SlicePlaneUniforms};
use polyscope_core::MaterialLoadRequest;
use polyscope_render::RenderEngine;

/// Auto-fit camera to scene bounding box on first render.
/// Returns updated `camera_fitted` value.
pub(super) fn auto_fit_camera(engine: &mut RenderEngine, camera_fitted: bool) -> bool {
    // Copy from render.rs lines 22-37 (identical in render() and render_frame_headless())
}

/// Drain and process the deferred material load queue.
pub(super) fn drain_material_queue(engine: &mut RenderEngine) {
    // Copy from render.rs lines 39-57 (identical in both paths)
}

/// Initialize GPU resources for all structures in the registry.
/// This is the shared subset used by both windowed and headless paths.
/// Handles: PointCloud, SurfaceMesh (+ shadow + vector quantities),
/// CurveNetwork (+ tubes + nodes), CameraView, VolumeMesh.
pub(super) fn init_structure_gpu_resources(engine: &mut RenderEngine) {
    // Copy the common init logic from render.rs lines 67-637
    // For windowed-only features (pick init, VolumeGrid quantities), see
    // init_pick_resources() and init_volume_grid_quantities() below.
}

/// Update GPU buffers for all structures.
pub(super) fn update_gpu_buffers(engine: &RenderEngine) {
    // Copy from render.rs lines 639-708 (nearly identical in both paths)
}
```

**Step 2: Add module declaration**

Add `mod render_init;` to `crates/polyscope/src/app/mod.rs`.

**Step 3: Build**

```bash
cargo build 2>&1 | tail -5
```

**Step 4: Commit**

```bash
git add -A && git commit -m "refactor: extract shared GPU init into render_init.rs"
```

---

## Task 5: Wire `render_init.rs` into both render paths

**Files:**
- Modify: `crates/polyscope/src/app/render.rs` — both `render()` and `render_frame_headless()`

**Step 1: Replace duplicated init code in `render()`**

Replace lines 22-37 (camera fit), 39-57 (material queue), 59-65 (uniform updates), 639-708 (buffer updates) with calls to:

```rust
self.camera_fitted = render_init::auto_fit_camera(engine, self.camera_fitted);
render_init::drain_material_queue(engine);
engine.update_camera_uniforms();
crate::with_context(|ctx| {
    engine.update_slice_plane_uniforms(ctx.slice_planes().map(SlicePlaneUniforms::from));
});
// ... GPU init (keep windowed-specific parts inline for now) ...
render_init::update_gpu_buffers(engine);
```

**Step 2: Replace duplicated init code in `render_frame_headless()`**

Replace lines 2820-2832 (camera fit), 2834-2852 (material queue), 2854-2860 (uniform updates), 2862-3067 (GPU init), 3069-3117 (buffer updates) with calls to the shared helpers.

**Step 3: Build and test**

```bash
cargo build 2>&1 | tail -5
cargo test 2>&1 | tail -10
cargo clippy 2>&1 | tail -5
```

**Step 4: Commit**

```bash
git add -A && git commit -m "refactor: dedup GPU init/update via render_init helpers"
```

---

## Task 6: Create `render_ui.rs` — extract egui integration

**Files:**
- Create: `crates/polyscope/src/app/render_ui.rs`
- Modify: `crates/polyscope/src/app/mod.rs` (add `mod render_ui;`)
- Modify: `crates/polyscope/src/app/render.rs`

**Step 1: Define UiResult struct and extract UI building**

```rust
// crates/polyscope/src/app/render_ui.rs
//! egui UI integration: panels, gizmos, settings synchronization.

use super::*;

/// Result of UI building, containing flags for what changed.
pub(super) struct UiResult {
    pub camera_changed: bool,
    pub scene_extents_changed: bool,
    pub screenshot_requested: bool,
    pub reset_view_requested: bool,
    pub ssaa_changed: bool,
    pub egui_output: egui::FullOutput,
}

impl App {
    /// Build the egui UI for one frame. Returns flags indicating what changed.
    pub(super) fn build_ui(&mut self, engine: &mut RenderEngine, window: &std::sync::Arc<winit::window::Window>) -> UiResult {
        // Move lines 879-1450 from render() here
        // This includes:
        //   - Multi-pass egui layout loop
        //   - Left panel with all sections (controls, camera, scene extents,
        //     appearance, tone mapping, materials, slice planes, gizmos, groups, structures)
        //   - Selection panel
        //   - Transform gizmo interaction
        //   - Slice plane gizmo interaction
        //   - Settings sync (background color, ground plane, camera, scene extents, SSAA)
    }
}
```

**Step 2: Replace inline UI code in `render()` with call to `build_ui()`**

```rust
let egui = self.egui.as_mut().unwrap();
let ui_result = self.build_ui(engine, window);
// Apply ui_result flags...
```

**NOTE:** There's a borrow-checker challenge here. `self.build_ui()` borrows `self` mutably, but `engine` and `window` are already borrowed from `self`. The solution is to destructure the borrows before calling, or pass the individual fields. This requires careful handling — see the actual implementation for the right pattern (likely passing `&mut EguiIntegration` + needed `App` fields explicitly).

**Step 3: Build and test**

```bash
cargo build 2>&1 | tail -5
cargo test 2>&1 | tail -10
cargo clippy 2>&1 | tail -5
```

**Step 4: Commit**

```bash
git add -A && git commit -m "refactor: extract egui UI integration into render_ui.rs"
```

---

## Task 7: Create `render_capture.rs` — extract screenshot + headless

**Files:**
- Create: `crates/polyscope/src/app/render_capture.rs`
- Modify: `crates/polyscope/src/app/mod.rs` (add `mod render_capture;`)
- Modify: `crates/polyscope/src/app/render.rs` (remove moved functions)

**Step 1: Move capture functions to `render_capture.rs`**

Move these functions from `render.rs`:
- `capture_screenshot()` (already refactored in Task 3 to use render_scene helpers)
- `render_frame_headless()` (already refactored in Task 5 to use render_init helpers)
- `capture_screenshot_headless()` (already refactored in Task 3)
- `capture_to_buffer()`

```rust
// crates/polyscope/src/app/render_capture.rs
//! Screenshot capture and headless rendering.

use super::*;

impl App {
    pub(super) fn capture_screenshot(&mut self, filename: String) { ... }
    pub(crate) fn render_frame_headless(&mut self) { ... }
    fn capture_screenshot_headless(&mut self) { ... }
    pub(crate) fn capture_to_buffer(&mut self) -> crate::Result<Vec<u8>> { ... }
}
```

**Step 2: Remove moved functions from `render.rs`**

Delete the function bodies that were moved. `render.rs` should now only contain:
- `render()` — the main windowed render orchestrator
- Import declarations

**Step 3: Build and test**

```bash
cargo build 2>&1 | tail -5
cargo test 2>&1 | tail -10
cargo clippy 2>&1 | tail -5
```

**Step 4: Commit**

```bash
git add -A && git commit -m "refactor: extract screenshot/headless into render_capture.rs"
```

---

## Task 8: Final cleanup and verification

**Files:**
- Modify: All new files (cleanup imports, fix warnings)

**Step 1: Run full check suite**

```bash
cargo fmt
cargo build 2>&1 | tail -5
cargo test 2>&1 | tail -10
cargo clippy 2>&1 | tail -5
```

Fix any warnings or unused imports.

**Step 2: Verify file sizes**

```bash
find crates/polyscope/src/app/ -name "*.rs" -exec wc -l {} + | sort -rn
```

Expected: No file exceeds ~900 lines. Total line count should be ~2,750 (down from 3,459 due to dedup).

**Step 3: Run headless integration tests specifically**

```bash
cargo test headless -- --nocapture 2>&1 | tail -20
```

Headless rendering is the most likely path to break since it had the most duplication.

**Step 4: Final commit**

```bash
git add -A && git commit -m "refactor: final cleanup after render.rs split"
```

---

## Implementation Order & Dependencies

```
Task 1 (render_scene.rs)  ──→  Task 2 (wire into render)  ──→  Task 3 (wire into screenshot/headless)
                                                                         │
Task 4 (render_init.rs)   ──→  Task 5 (wire into both paths)           │
                                                                         │
                                   Task 6 (render_ui.rs)  ──────────────┤
                                                                         │
                                   Task 7 (render_capture.rs)  ←────────┘
                                                                         │
                                   Task 8 (final cleanup)  ←────────────┘
```

Tasks 1→2→3 and 4→5 can be done in parallel. Task 6 and 7 depend on earlier tasks being done. Task 8 is always last.

## Key Risk: Borrow Checker

The main risk is wgpu lifetime management. `wgpu::RenderPass<'a>` borrows the encoder, and bind groups/buffers must outlive the pass. The shared draw functions need lifetime annotations:

```rust
pub(super) fn draw_point_clouds<'a>(
    render_pass: &mut wgpu::RenderPass<'a>,
    engine: &'a RenderEngine,
)
```

The `'a` lifetime ties the render pass to the engine's resources. This pattern is standard in wgpu code and should work cleanly.

For Task 6 (UI extraction), the borrow checker requires careful handling since `App` fields (engine, egui, window) are borrowed simultaneously. The solution is to pass individual fields rather than `&mut self`.
