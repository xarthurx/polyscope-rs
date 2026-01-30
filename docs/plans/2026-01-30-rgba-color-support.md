# RGBA Color Support Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add RGBA color support throughout polyscope-rs, replacing Vec3 color storage with Vec4 and propagating alpha to GPU buffers and UI.

**Architecture:** Change all internal color storage from `Vec3` to `Vec4` (with alpha channel). Keep existing `Vec3` public API methods working by extending them with `alpha=1.0` default, and add new `_rgba` setter variants. GPU uniforms already use `[f32; 4]` so the main work is CPU-side storage and buffer creation.

**Tech Stack:** glam (Vec3/Vec4), wgpu (GPU buffers), egui (UI color pickers), WGSL shaders

---

### Design Decisions

1. **Internal storage**: All color fields change from `Vec3` to `Vec4`
2. **Public API**: Existing `set_*_color(Vec3)` methods stay (auto alpha=1.0). No new `_rgba` methods needed initially since users can use `with_*` closures for Vec4 access.
3. **Color quantities**: `Vec<Vec3>` → `Vec<Vec4>` for color quantity data
4. **GPU buffers**: Stop hardcoding `1.0` alpha, use stored `.w` component
5. **UI**: Change `color_edit_button_rgb` → `color_edit_button_srgba` where alpha matters, keep RGB for structure base colors (alpha controlled separately via transparency)
6. **WGSL shaders**: Update `array<vec3<f32>>` color storage buffers to `array<vec4<f32>>`

---

### Task 1: Core types — Options and SlicePlane

**Files:**
- Modify: `crates/polyscope-core/src/options.rs:30,57`
- Modify: `crates/polyscope-core/src/slice_plane.rs:27,46,61`

**Step 1: Update Options**

Change `background_color: Vec3` → `Vec4` at line 30. Update default at line 57 from `Vec3::new(1.0, 1.0, 1.0)` to `Vec4::new(1.0, 1.0, 1.0, 1.0)`.

**Step 2: Update SlicePlane**

Change `color: Vec3` → `Vec4` at line 27. Update defaults at lines 46, 61 from `Vec3::new(0.5, 0.5, 0.5)` to `Vec4::new(0.5, 0.5, 0.5, 1.0)`. Update getter/setter signatures.

**Step 3: Fix all compilation errors in polyscope-core**

Run: `cargo check -p polyscope-core`

Search for any remaining references to the old Vec3 types in this crate and fix them.

**Step 4: Commit**

```bash
git add crates/polyscope-core/
git commit -m "refactor(core): change color storage from Vec3 to Vec4 (RGBA)"
```

---

### Task 2: Point Cloud — base color and quantities

**Files:**
- Modify: `crates/polyscope-structures/src/point_cloud/mod.rs:24,43,193,199,264,268,312`
- Modify: `crates/polyscope-structures/src/point_cloud/quantities.rs:156,174,216,257,267,321,330`

**Step 1: Update PointCloud base_color**

- Line 24: `base_color: Vec3` → `Vec4`
- Line 43: default → `Vec4::new(0.2, 0.5, 0.8, 1.0)`
- Line 193: `set_base_color(&mut self, color: Vec3)` — change to accept `Vec3`, convert internally: `self.base_color = color.extend(1.0);`
- Line 199: `base_color() -> Vec3` — change return to `Vec4`
- Line 264: UI conversion — use `[self.base_color.x, self.base_color.y, self.base_color.z]` (keep RGB for UI picker)
- Line 268: reconstruction — `self.base_color = Vec4::new(color[0], color[1], color[2], self.base_color.w);`
- Line 312: GPU uniform — change from `[self.base_color.x, self.base_color.y, self.base_color.z, 1.0]` to `self.base_color.to_array()`

**Step 2: Update PointCloudVectorQuantity color**

- Line 156: `color: Vec3` → `Vec4`
- Line 174: default → `Vec4::new(0.8, 0.2, 0.2, 1.0)`
- Line 216: GPU uniform — `self.color.to_array()`
- Line 257: UI — keep `[self.color.x, self.color.y, self.color.z]`
- Line 267: UI reconstruction — `self.color = Vec4::new(color[0], color[1], color[2], self.color.w);`

**Step 3: Update PointCloudColorQuantity**

- Line 321: `colors: Vec<Vec3>` → `Vec<Vec4>`
- Line 330: constructor — accept `Vec<Vec3>`, convert: `colors.into_iter().map(|c| c.extend(1.0)).collect()`
- Update `colors()` getter return type

**Step 4: Build check**

Run: `cargo check -p polyscope-structures`

Fix any remaining compilation errors in point cloud code.

**Step 5: Commit**

```bash
git add crates/polyscope-structures/src/point_cloud/
git commit -m "refactor(point_cloud): change color storage from Vec3 to Vec4"
```

---

### Task 3: Surface Mesh — structure colors and quantities

**Files:**
- Modify: `crates/polyscope-structures/src/surface_mesh/mod.rs:69,72,73,108,111,112,255-306,498-507,1210-1227`
- Modify: `crates/polyscope-structures/src/surface_mesh/quantities.rs:295,304,316,370,379,391`

**Step 1: Update SurfaceMesh color fields**

- Lines 69,72,73: `edge_color/backface_color/surface_color: Vec3` → `Vec4`
- Lines 108,111,112: defaults → `Vec4` with alpha 1.0
- Lines 255-306: getter/setter methods — setters accept Vec3 and extend(1.0), getters return Vec4
- Lines 498-507: UI conversions — keep `[f32; 3]` for UI, reconstruct with preserved alpha
- Lines 1210-1227: GPU uniforms — use `.to_array()` instead of manual `[x, y, z, 1.0]`

**Step 2: Update color quantity types**

- MeshVertexColorQuantity (line 295): `colors: Vec<Vec3>` → `Vec<Vec4>`
- Constructor (line 304): convert from Vec3 input
- Getter (line 316): return `&[Vec4]`
- MeshFaceColorQuantity (line 370): `colors: Vec<Vec3>` → `Vec<Vec4>` (note: stored per-corner)
- Constructor (line 379): convert from Vec3 input
- Getter (line 391): return `&[Vec4]`

**Step 3: Update intrinsic vector and one-form color fields**

Check `intrinsic_vector_quantity.rs` and `one_form_quantity.rs` for Vec3 color fields and convert similarly.

**Step 4: Build check**

Run: `cargo check -p polyscope-structures`

**Step 5: Commit**

```bash
git add crates/polyscope-structures/src/surface_mesh/
git commit -m "refactor(surface_mesh): change color storage from Vec3 to Vec4"
```

---

### Task 4: Curve Network — structure color and quantities

**Files:**
- Modify: `crates/polyscope-structures/src/curve_network/mod.rs:37,79,174-182,362,377,630,648`
- Modify: `crates/polyscope-structures/src/curve_network/quantities.rs:293,302,314,377,386,398`

**Step 1: Update CurveNetwork color field**

- Line 37: `color: Vec3` → `Vec4`
- Line 79: default → `Vec4::new(0.2, 0.5, 0.8, 1.0)`
- Lines 174-182: getter returns Vec4, setter accepts Vec3 and extends
- Lines 362,377: UI conversions — keep [f32;3] for picker
- Lines 630,648: GPU uniforms — use `.to_array()`

**Step 2: Update color quantity types**

- CurveNodeColorQuantity (line 293): `Vec<Vec3>` → `Vec<Vec4>`
- CurveEdgeColorQuantity (line 377): `Vec<Vec3>` → `Vec<Vec4>`
- Constructors and getters updated

**Step 3: Build check**

Run: `cargo check -p polyscope-structures`

**Step 4: Commit**

```bash
git add crates/polyscope-structures/src/curve_network/
git commit -m "refactor(curve_network): change color storage from Vec3 to Vec4"
```

---

### Task 5: Volume Mesh and Volume Grid colors

**Files:**
- Modify: `crates/polyscope-structures/src/volume_mesh/mod.rs:88-90,113,115,126`
- Modify: `crates/polyscope-structures/src/volume_mesh/color_quantity.rs:10,18,29,80,88,99`
- Modify: `crates/polyscope-structures/src/volume_grid/mod.rs:35-36,61-62`

**Step 1: Update VolumeMesh**

- Lines 88-90: `color/interior_color/edge_color: Vec3` → `Vec4`
- Lines 113,115,126: defaults with alpha 1.0
- Update setters to accept Vec3 and extend, update GPU uniform construction

**Step 2: Update volume color quantities**

- VolumeMeshVertexColorQuantity (line 10): `Vec<Vec3>` → `Vec<Vec4>`
- VolumeMeshCellColorQuantity (line 80): `Vec<Vec3>` → `Vec<Vec4>`

**Step 3: Update VolumeGrid**

- Lines 35-36: `color/edge_color: Vec3` → `Vec4`
- Lines 61-62: defaults with alpha 1.0

**Step 4: Build check**

Run: `cargo check -p polyscope-structures`

**Step 5: Commit**

```bash
git add crates/polyscope-structures/src/volume_mesh/ crates/polyscope-structures/src/volume_grid/
git commit -m "refactor(volume): change color storage from Vec3 to Vec4"
```

---

### Task 6: Floating quantities and Camera View

**Files:**
- Modify: `crates/polyscope-structures/src/floating/color_image.rs`
- Modify: `crates/polyscope-structures/src/floating/render_image.rs`
- Modify: `crates/polyscope-structures/src/camera_view/mod.rs:251`

**Step 1: Update floating color image types**

Change color storage from Vec3 to Vec4 in FloatingColorImage, FloatingColorRenderImage, FloatingRawColorImage.

**Step 2: Update CameraView color**

Line 251: color conversion to Vec4.

**Step 3: Build check**

Run: `cargo check -p polyscope-structures`

**Step 4: Commit**

```bash
git add crates/polyscope-structures/src/floating/ crates/polyscope-structures/src/camera_view/
git commit -m "refactor(floating,camera): change color storage from Vec3 to Vec4"
```

---

### Task 7: Render module — GPU buffer creation

**Files:**
- Modify: `crates/polyscope-render/src/point_cloud_render.rs:74,127`
- Modify: `crates/polyscope-render/src/surface_mesh_render.rs:354,364`
- Modify: `crates/polyscope-render/src/curve_network_render.rs` (color buffer creation)
- Modify: `crates/polyscope-render/src/slice_mesh_render.rs` (color buffer creation)

**Step 1: Update all color buffer creation**

Replace all instances of:
```rust
colors.iter().flat_map(|c| [c.x, c.y, c.z, 1.0]).collect()
```
With:
```rust
colors.iter().flat_map(|c| c.to_array()).collect()
```

Since colors are now Vec4, this naturally includes the alpha.

**Step 2: Build check**

Run: `cargo check -p polyscope-render`

**Step 3: Commit**

```bash
git add crates/polyscope-render/src/
git commit -m "refactor(render): use Vec4 alpha from color storage instead of hardcoded 1.0"
```

---

### Task 8: WGSL Shaders — color storage buffers

**Files:**
- Modify: `crates/polyscope-render/src/shaders/point_sphere.wgsl:36`
- Modify: `crates/polyscope-render/src/shaders/reflected_point_sphere.wgsl:31`

**Step 1: Update shader declarations**

Change `array<vec3<f32>>` to `array<vec4<f32>>` for color storage buffers.

Update vertex shader code that reads from these buffers to use `.xyz` or full `.xyzw` as needed.

**Step 2: Build and run test**

Run: `cargo build`

**Step 3: Commit**

```bash
git add crates/polyscope-render/src/shaders/
git commit -m "refactor(shaders): update color storage buffers from vec3 to vec4"
```

---

### Task 9: UI module — color pickers

**Files:**
- Modify: `crates/polyscope-ui/src/quantity_ui.rs:132,220,234,295`
- Modify: `crates/polyscope-ui/src/structure_ui.rs:41,112,145,211`

**Step 1: Update UI color handling**

The UI color pickers should continue using RGB for structure base colors (alpha is controlled via the separate transparency slider). Color quantity UIs can stay RGB since per-element alpha is the Vec4 data.

Update the conversion code in structure UIs to work with Vec4 fields:
- Read: `let mut c = [field.x, field.y, field.z];`
- Write back: `field = Vec4::new(c[0], c[1], c[2], field.w);`

Keep `color_edit_button_rgb` calls — no need to change to RGBA pickers since alpha is either 1.0 or controlled by transparency.

**Step 2: Build check**

Run: `cargo check -p polyscope-ui`

**Step 3: Commit**

```bash
git add crates/polyscope-ui/src/
git commit -m "refactor(ui): update color picker code for Vec4 storage"
```

---

### Task 10: Public API — lib.rs handle methods

**Files:**
- Modify: `crates/polyscope/src/lib.rs`

**Step 1: Update handle methods**

Update `SurfaceMeshHandle`, `CurveNetworkHandle`, `PointCloudHandle` color methods to continue accepting `Vec3` (the underlying structs' setters handle the conversion).

Update any public free functions that take/return color Vec3 to Vec4 where appropriate.

**Step 2: Build full project**

Run: `cargo build`
Run: `cargo test`
Run: `cargo clippy`

**Step 3: Commit**

```bash
git add crates/polyscope/
git commit -m "refactor(api): update public API for Vec4 color support"
```

---

### Task 11: Update examples and documentation

**Files:**
- Modify: `examples/*.rs` (any that reference color types directly)
- Modify: `CLAUDE.md`
- Modify: `README.md`
- Modify: `docs/architecture-differences.md`

**Step 1: Fix example compilation**

Run: `cargo build --examples`

Fix any examples that break due to Vec3→Vec4 changes. Most examples use `Vec3` for colors passed to setters, which should still work since setters accept Vec3.

**Step 2: Update documentation**

- README.md: Remove "Color RGBA (currently RGB only)" from "What's Not Yet Implemented"
- CLAUDE.md: Update "Missing Features" section
- architecture-differences.md: Note RGBA support

**Step 3: Final verification**

Run: `cargo build --examples && cargo test && cargo clippy`

**Step 4: Commit**

```bash
git add .
git commit -m "feat: complete RGBA color support across all structures"
```

---

### Risk Notes

- **Breaking change**: `base_color()`, `surface_color()`, etc. return `Vec4` instead of `Vec3`. Code using `.x/.y/.z` still works, but `let c: Vec3 = mesh.surface_color()` will fail.
- **Mitigation**: The public setters still accept `Vec3`. Only getters change return type.
- **WGSL alignment**: `array<vec4<f32>>` has different stride than `array<vec3<f32>>`. Verify GPU buffer sizes match after shader changes.
