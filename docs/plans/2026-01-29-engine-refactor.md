# engine.rs Refactor: Split into Focused Modules

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Split the 4,387-line `engine.rs` into 6 focused modules to improve AI coding ergonomics and maintainability.

**Architecture:** Convert `engine.rs` (single file) to `engine/` (module directory). The `RenderEngine` struct stays in `mod.rs`. Method implementations are split across sibling files using separate `impl RenderEngine` blocks. All fields use `pub(crate)` visibility so sibling modules within the crate can access them.

**Tech Stack:** Rust module system, no new dependencies.

**Worktree:** `.worktrees/refactor-engine` (branch: `refactor/split-engine-rs`)

---

## Module Assignment

### `engine/mod.rs` (~500 lines)
- Struct definition (lines 23-221)
- `CameraUniforms` struct + Default (lines 23-43)
- `new_windowed()` (225-547)
- `new_headless()` (550-857)
- `resize()` (860-902)
- `create_depth_texture()` (904-934) — static helper
- `update_camera_uniforms()` (1053-1070)
- `update_slice_plane_uniforms()` (1076-1084)
- `dimensions()` (2998-3002)
- `render_dimensions()` (3205-3209)
- Core accessor methods: `camera_buffer()`, `shadow_map_pass()`, `depth_view()`, `hdr_texture_view()`

### `engine/pipelines.rs` (~1,100 lines)
All pipeline creation functions (private):
- `init_point_pipeline()` (937-1050)
- `init_vector_pipeline()` (1104-1219)
- `create_mesh_pipeline()` (1234-1389)
- `create_curve_network_edge_pipeline()` (1399-1535)
- `create_curve_network_tube_pipelines()` (1537-1756)
- `create_shadow_pipeline()` (1780-1887)
- `create_ground_stencil_pipeline()` (1898-1970)
- `create_reflected_mesh_pipeline()` (1971-2147)
- `create_reflected_point_cloud_pipeline()` (2148-2288)
- `create_reflected_curve_network_pipeline()` (2289-2464)
- `create_mesh_oit_pipeline()` (3291-3388)
- Pipeline accessor methods: `point_bind_group_layout()`, `vector_bind_group_layout()`, `mesh_bind_group_layout()`, `curve_network_*_layout()`, `shadow_pipeline()`, `shadow_bind_group_layout()`, `mesh_oit_pipeline()`, `ensure_mesh_oit_pipeline()`

### `engine/rendering.rs` (~600 lines)
Ground plane, slice planes, stencil, reflection rendering:
- `render_ground_plane()` (2465-2546)
- `render_slice_planes()` (2552-2614)
- `render_slice_planes_with_clear()` (2615-2717)
- `render_stencil_pass()` (2718-2806)
- `create_reflected_mesh_bind_group()` (3663-3707)
- `render_reflected_mesh()` (3708-3728)
- `create_reflected_point_cloud_bind_group()` (3729-3759)
- `render_reflected_point_cloud()` (3760-3781)
- `create_reflected_curve_network_bind_group()` (3782-3812)
- `render_reflected_curve_network()` (3813-3840)
- `update_reflection()` (3651-3662)
- `reflection_pass()` (3646-3650)
- `init_reflection_pass()` (3641-3645)

### `engine/postprocessing.rs` (~500 lines)
SSAO, tone mapping, OIT, SSAA, screenshot:
- `init_tone_mapping()` (3003-3011)
- `init_ssao_pass()` (3012-3018)
- `init_ssaa_pass()` (3043-3051)
- `create_ssao_output_texture()` (3019-3042)
- `ssaa_factor()` (3052-3057)
- `set_ssaa_factor()` (3058-3078)
- `recreate_ssaa_textures()` (3079-3113)
- `create_ssaa_intermediate_texture()` (3114-3135)
- `render_ssao()` (3530-3584)
- `render_tone_mapping()` (3610-3640)
- `update_tone_mapping()` (3590-3609)
- `tone_map_pass()` (3585-3589)
- `ssao_pass()` (3524-3529)
- SSAO view accessors: `hdr_view()`, `normal_view()`, `ssao_noise_view()`, `ssao_output_view()`
- OIT: `ensure_oit_textures()` (3210-3258), `ensure_oit_pass()` (3269-3279), `oit_composite_pass()`, `oit_accum_view()`, `oit_reveal_view()`
- Screenshot: `create_screenshot_target()` (2807-2863), `screenshot_texture_view()`, `apply_screenshot_tone_mapping()` (2871-2901), `screenshot_depth_view()`, `capture_screenshot()` (2920-2997), `aligned_bytes_per_row()` (2907-2918)

### `engine/textures.rs` (~300 lines)
Texture and resource creation helpers:
- `create_hdr_texture()` (3397-3418)
- `create_normal_texture()` (3420-3441)
- `create_ssao_noise_texture()` (3443-3502)
- `create_hdr_texture_with_size()` (3136-3157)
- `create_normal_texture_with_size()` (3158-3179)
- `create_ssao_output_texture_with_size()` (3180-3204)

### `engine/pick.rs` (~500 lines)
Pick system (structure IDs + GPU picking):
- `assign_structure_id()` (3853-3865)
- `remove_structure_id()` (3866-3873)
- `lookup_structure_id()` (3874-3880)
- `get_structure_id()` (3881-3888)
- `init_pick_buffers()` (3889-3949)
- `init_pick_pipeline()` (3950-4050)
- `pick_bind_group_layout()` (4051-4057)
- `point_pick_pipeline()` (4058-4064)
- `curve_network_pick_pipeline()` (4065-4071)
- `init_curve_network_pick_pipeline()` (4072-4134)
- `has_curve_network_pick_pipeline()` (4135-4139)
- `init_curve_network_tube_pick_pipeline()` (4140-4259)
- `has_curve_network_tube_pick_pipeline()` (4260-4264)
- `curve_network_tube_pick_pipeline()` (4265-4271)
- `curve_network_tube_pick_bind_group_layout()` (4272-4280)
- `pick_at()` (4282-4343)
- `pick_texture_view()` (4344-4348)
- `pick_depth_view()` (4349-4355)
- `begin_pick_pass()` (4357-4385)

---

## Implementation Tasks

### Task 1: Create engine/ directory structure

**Files:**
- Rename: `crates/polyscope-render/src/engine.rs` → `crates/polyscope-render/src/engine/mod.rs`
- Create: `crates/polyscope-render/src/engine/pipelines.rs` (empty)
- Create: `crates/polyscope-render/src/engine/rendering.rs` (empty)
- Create: `crates/polyscope-render/src/engine/postprocessing.rs` (empty)
- Create: `crates/polyscope-render/src/engine/textures.rs` (empty)
- Create: `crates/polyscope-render/src/engine/pick.rs` (empty)

**Step 1:** Rename `engine.rs` to `engine/mod.rs`
```bash
cd crates/polyscope-render/src
mkdir engine
git mv engine.rs engine/mod.rs
```

**Step 2:** Create empty submodule files with module declarations
Each file starts with the necessary `use` imports and an `impl RenderEngine { }` block.

**Step 3:** Add module declarations to `engine/mod.rs`
```rust
mod pipelines;
mod rendering;
mod postprocessing;
mod textures;
mod pick;
```

**Step 4:** Build to confirm no breakage
```bash
cargo build
```
Expected: Clean build (file rename is transparent to Rust module system)

**Step 5:** Run tests
```bash
cargo test
```
Expected: All 148 tests pass

**Step 6:** Commit
```bash
git add -A
git commit -m "refactor: convert engine.rs to engine/ module directory"
```

---

### Task 2: Change field visibility to pub(crate)

**Files:**
- Modify: `crates/polyscope-render/src/engine/mod.rs`

**Step 1:** Change all `pub` fields on `RenderEngine` to `pub(crate)`, and all private fields to `pub(crate)`. This allows sibling modules (pipelines.rs, rendering.rs, etc.) to access fields.

Note: Fields that are currently accessed from outside the crate (e.g., from `polyscope/src/app.rs`) use `pub` which is fine — `pub(crate)` would break them. Keep those as `pub`. The private fields (no visibility modifier) need `pub(crate)`.

Key private fields that need `pub(crate)`:
- `depth_only_view`
- `mesh_bind_group_layout`
- `curve_network_edge_bind_group_layout`
- `curve_network_tube_bind_group_layout`
- `curve_network_tube_compute_bind_group_layout`
- `ground_plane_pipeline`, `ground_plane_bind_group_layout`, `ground_plane_render_data`
- `slice_plane_vis_pipeline`, `slice_plane_vis_bind_group_layout`, `slice_plane_render_data`
- `screenshot_*` fields
- `hdr_*`, `normal_*`, `ssao_*`, `oit_*` fields
- `tone_map_pass`, `ssaa_*`, `shadow_*`
- `reflection_pass`, `ground_stencil_pipeline`, `reflected_*`
- All pick fields (structure_id_map, pick_*, etc.)

**Step 2:** Build to confirm
```bash
cargo build
```

**Step 3:** Commit
```bash
git commit -am "refactor: change RenderEngine field visibility to pub(crate)"
```

---

### Task 3: Extract pick.rs

**Files:**
- Modify: `crates/polyscope-render/src/engine/mod.rs` (remove functions)
- Modify: `crates/polyscope-render/src/engine/pick.rs` (add functions)

**Step 1:** Move all pick-related functions (lines 3853-4385) to `engine/pick.rs`. Add necessary imports at top of pick.rs.

**Step 2:** Build and test
```bash
cargo build && cargo test
```

**Step 3:** Commit
```bash
git commit -am "refactor: extract pick system to engine/pick.rs"
```

---

### Task 4: Extract textures.rs

**Files:**
- Modify: `crates/polyscope-render/src/engine/mod.rs` (remove functions)
- Modify: `crates/polyscope-render/src/engine/textures.rs` (add functions)

**Step 1:** Move texture creation functions to `engine/textures.rs`:
- `create_hdr_texture()`, `create_normal_texture()`, `create_ssao_noise_texture()`
- `create_hdr_texture_with_size()`, `create_normal_texture_with_size()`, `create_ssao_output_texture_with_size()`

**Step 2:** Build and test

**Step 3:** Commit
```bash
git commit -am "refactor: extract texture creation to engine/textures.rs"
```

---

### Task 5: Extract postprocessing.rs

**Files:**
- Modify: `crates/polyscope-render/src/engine/mod.rs` (remove functions)
- Modify: `crates/polyscope-render/src/engine/postprocessing.rs` (add functions)

**Step 1:** Move post-processing, SSAO, OIT, SSAA, screenshot, and tone mapping functions.

**Step 2:** Build and test

**Step 3:** Commit
```bash
git commit -am "refactor: extract post-processing to engine/postprocessing.rs"
```

---

### Task 6: Extract rendering.rs

**Files:**
- Modify: `crates/polyscope-render/src/engine/mod.rs` (remove functions)
- Modify: `crates/polyscope-render/src/engine/rendering.rs` (add functions)

**Step 1:** Move ground plane, slice plane, stencil, and reflection rendering functions.

**Step 2:** Build and test

**Step 3:** Commit
```bash
git commit -am "refactor: extract rendering passes to engine/rendering.rs"
```

---

### Task 7: Extract pipelines.rs

**Files:**
- Modify: `crates/polyscope-render/src/engine/mod.rs` (remove functions)
- Modify: `crates/polyscope-render/src/engine/pipelines.rs` (add functions)

**Step 1:** Move all pipeline creation functions and their accessor methods.

**Step 2:** Build and test

**Step 3:** Commit
```bash
git commit -am "refactor: extract pipeline creation to engine/pipelines.rs"
```

---

### Task 8: Final verification

**Step 1:** Run full test suite
```bash
cargo test
```
Expected: All 148 tests pass

**Step 2:** Run clippy
```bash
cargo clippy
```
Expected: Clean (matching pre-refactor warnings only)

**Step 3:** Verify mod.rs is now ~500 lines
```bash
wc -l crates/polyscope-render/src/engine/mod.rs
```

**Step 4:** Final commit if any cleanup needed
