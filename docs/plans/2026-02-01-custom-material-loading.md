# Custom Material Loading Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add runtime loading of user-provided matcap textures from disk, with both programmatic API and UI panel.

**Architecture:** New `decode_matcap_image_from_file()` function in `materials.rs` loads images via `image::open()`, then feeds into the existing `upload_matcap_texture()` pipeline. `RenderEngine` gets `load_blendable_material()` and `load_static_material()` methods. Public API in `polyscope` crate wraps these. UI material selector becomes dynamic from registry; new "Materials" collapsing section added to left panel.

**Tech Stack:** Rust, wgpu, egui, `image` crate (already a dependency)

---

### Task 1: Add `MaterialExists` error variant

**Files:**
- Modify: `crates/polyscope-core/src/error.rs:7-47`

**Step 1: Add error variant**

In `PolyscopeError` enum, add after the `QuantityNotFound` variant (line 30):

```rust
    /// A material with the given name already exists.
    #[error("material '{0}' already exists")]
    MaterialExists(String),

    /// Failed to load a material image.
    #[error("material load error: {0}")]
    MaterialLoadError(String),
```

**Step 2: Verify it compiles**

Run: `cargo check -p polyscope-core`
Expected: OK (new variants are unused but that's fine)

**Step 3: Commit**

```bash
git add crates/polyscope-core/src/error.rs
git commit -m "feat(materials): add MaterialExists and MaterialLoadError error variants"
```

---

### Task 2: Add `decode_matcap_image_from_file()` to materials.rs

**Files:**
- Modify: `crates/polyscope-render/src/materials.rs:321-345`

**Step 1: Add the file-based decode function**

Add after the existing `decode_matcap_image()` function (after line 345):

```rust
/// Decode an image file from disk into float RGBA pixel data.
///
/// Returns `(width, height, rgba_f32_pixels)` where pixels are laid out as
/// `[r, g, b, a, r, g, b, a, ...]` in linear float space.
///
/// Supports any format the `image` crate can open: HDR, JPEG, PNG, EXR, etc.
pub fn decode_matcap_image_from_file(
    path: &std::path::Path,
) -> std::result::Result<(u32, u32, Vec<f32>), String> {
    use image::GenericImageView;

    let img = image::open(path).map_err(|e| format!("failed to open '{}': {}", path.display(), e))?;
    let (width, height) = img.dimensions();

    if width == 0 || height == 0 {
        return Err(format!("image '{}' has zero dimensions", path.display()));
    }

    let rgb32f = img.to_rgb32f();
    let pixels = rgb32f.as_raw();

    // Pad RGB -> RGBA with alpha=1.0
    let mut rgba = Vec::with_capacity((width * height * 4) as usize);
    for chunk in pixels.chunks(3) {
        rgba.push(chunk[0]);
        rgba.push(chunk[1]);
        rgba.push(chunk[2]);
        rgba.push(1.0);
    }

    Ok((width, height, rgba))
}
```

**Step 2: Make `upload_matcap_texture` and `create_matcap_sampler` public**

Change visibility on lines 348 and 401:
- `fn upload_matcap_texture(` → `pub fn upload_matcap_texture(`
- `fn create_matcap_sampler(` → `pub fn create_matcap_sampler(`

**Step 3: Verify it compiles**

Run: `cargo check -p polyscope-render`
Expected: OK

**Step 4: Commit**

```bash
git add crates/polyscope-render/src/materials.rs
git commit -m "feat(materials): add decode_matcap_image_from_file and make upload helpers public"
```

---

### Task 3: Add `has_material()` to MaterialRegistry

**Files:**
- Modify: `crates/polyscope-render/src/materials.rs:202-284`

**Step 1: Add method**

Add after the existing `get()` method (after line 242):

```rust
    /// Returns true if a material with the given name is registered.
    #[must_use]
    pub fn has(&self, name: &str) -> bool {
        self.materials.contains_key(name)
    }
```

**Step 2: Add `sorted_names()` method**

Replace the existing `names()` method (lines 264-271) with one that returns built-ins first, then custom materials sorted alphabetically:

```rust
    /// Returns all material names, with built-in materials first in a stable order,
    /// followed by custom materials sorted alphabetically.
    #[must_use]
    pub fn names(&self) -> Vec<&str> {
        const BUILTIN_ORDER: &[&str] = &[
            "clay", "wax", "candy", "flat", "mud", "ceramic", "jade", "normal",
        ];
        let mut names: Vec<&str> = Vec::new();
        // Built-ins first, in canonical order
        for &builtin in BUILTIN_ORDER {
            if self.materials.contains_key(builtin) {
                names.push(builtin);
            }
        }
        // Custom materials sorted alphabetically
        let mut custom: Vec<&str> = self
            .materials
            .keys()
            .map(String::as_str)
            .filter(|n| !BUILTIN_ORDER.contains(n))
            .collect();
        custom.sort();
        names.extend(custom);
        names
    }
```

**Step 3: Verify it compiles and tests pass**

Run: `cargo test -p polyscope-render -- materials`
Expected: All existing material tests pass

**Step 4: Commit**

```bash
git add crates/polyscope-render/src/materials.rs
git commit -m "feat(materials): add has() and stable-ordered names() to MaterialRegistry"
```

---

### Task 4: Add loading methods on RenderEngine

**Files:**
- Modify: `crates/polyscope-render/src/engine/mod.rs` (add methods at end of `impl RenderEngine`)

**Step 1: Add import**

At the top of `engine/mod.rs`, add to the existing materials import (line 21):

```rust
use crate::materials::{self, MatcapTextureSet, MaterialRegistry, Material,
    create_matcap_sampler, upload_matcap_texture, decode_matcap_image_from_file};
```

**Step 2: Add `load_blendable_material` method**

Add before the closing `}` of `impl RenderEngine` (before the line after `render_dimensions()`):

```rust
    /// Loads a blendable (4-channel RGB-tintable) material from disk.
    ///
    /// Takes 4 image file paths for R, G, B, K matcap channels.
    /// Supports HDR, JPEG, PNG, EXR, and other formats via the `image` crate.
    pub fn load_blendable_material(
        &mut self,
        name: &str,
        filenames: [&str; 4],
    ) -> std::result::Result<(), polyscope_core::PolyscopeError> {
        use polyscope_core::PolyscopeError;

        if self.matcap_textures.contains_key(name) {
            return Err(PolyscopeError::MaterialExists(name.to_string()));
        }

        let channel_labels = ["r", "g", "b", "k"];
        let mut views = Vec::with_capacity(4);

        for (i, filename) in filenames.iter().enumerate() {
            let path = std::path::Path::new(filename);
            let (w, h, rgba) = decode_matcap_image_from_file(path)
                .map_err(PolyscopeError::MaterialLoadError)?;
            let tex = upload_matcap_texture(
                &self.device,
                &self.queue,
                &format!("matcap_{name}_{}", channel_labels[i]),
                w,
                h,
                &rgba,
            );
            views.push(tex.create_view(&wgpu::TextureViewDescriptor::default()));
        }

        let sampler = create_matcap_sampler(&self.device);

        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(&format!("matcap_{name}_bind_group")),
            layout: &self.matcap_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&views[0]) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&views[1]) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::TextureView(&views[2]) },
                wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::TextureView(&views[3]) },
                wgpu::BindGroupEntry { binding: 4, resource: wgpu::BindingResource::Sampler(&sampler) },
            ],
        });

        // Drain views into individual variables
        let mut drain = views.into_iter();
        let tex_r = drain.next().unwrap();
        let tex_g = drain.next().unwrap();
        let tex_b = drain.next().unwrap();
        let tex_k = drain.next().unwrap();

        self.matcap_textures.insert(
            name.to_string(),
            MatcapTextureSet { tex_r, tex_g, tex_b, tex_k, sampler, bind_group },
        );

        self.materials.register(Material::blendable(name, 0.2, 0.7, 0.3, 32.0));

        Ok(())
    }

    /// Loads a static (single-texture, non-RGB-tintable) material from disk.
    ///
    /// The same texture is used for all 4 matcap channels.
    /// Supports HDR, JPEG, PNG, EXR, and other formats via the `image` crate.
    pub fn load_static_material(
        &mut self,
        name: &str,
        filename: &str,
    ) -> std::result::Result<(), polyscope_core::PolyscopeError> {
        use polyscope_core::PolyscopeError;

        if self.matcap_textures.contains_key(name) {
            return Err(PolyscopeError::MaterialExists(name.to_string()));
        }

        let path = std::path::Path::new(filename);
        let (w, h, rgba) = decode_matcap_image_from_file(path)
            .map_err(PolyscopeError::MaterialLoadError)?;
        let tex = upload_matcap_texture(
            &self.device,
            &self.queue,
            &format!("matcap_{name}"),
            w,
            h,
            &rgba,
        );

        let view_r = tex.create_view(&wgpu::TextureViewDescriptor::default());
        let view_g = tex.create_view(&wgpu::TextureViewDescriptor::default());
        let view_b = tex.create_view(&wgpu::TextureViewDescriptor::default());
        let view_k = tex.create_view(&wgpu::TextureViewDescriptor::default());

        let sampler = create_matcap_sampler(&self.device);

        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(&format!("matcap_{name}_bind_group")),
            layout: &self.matcap_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&view_r) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&view_g) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::TextureView(&view_b) },
                wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::TextureView(&view_k) },
                wgpu::BindGroupEntry { binding: 4, resource: wgpu::BindingResource::Sampler(&sampler) },
            ],
        });

        self.matcap_textures.insert(
            name.to_string(),
            MatcapTextureSet {
                tex_r: tex.create_view(&wgpu::TextureViewDescriptor::default()),
                tex_g: tex.create_view(&wgpu::TextureViewDescriptor::default()),
                tex_b: tex.create_view(&wgpu::TextureViewDescriptor::default()),
                tex_k: tex.create_view(&wgpu::TextureViewDescriptor::default()),
                sampler,
                bind_group,
            },
        );

        self.materials.register(Material::static_mat(name, 0.2, 0.7, 0.3, 32.0));

        Ok(())
    }
```

**Step 3: Verify it compiles**

Run: `cargo check -p polyscope-render`
Expected: OK

**Step 4: Commit**

```bash
git add crates/polyscope-render/src/engine/mod.rs
git commit -m "feat(materials): add load_blendable_material and load_static_material on RenderEngine"
```

---

### Task 5: Add public API wrappers in polyscope crate

**Files:**
- Modify: `crates/polyscope/src/lib.rs` (add functions after `clear_file_drop_callback`)

**Step 1: Add the three public API functions**

Add after `clear_file_drop_callback()` (after line 188):

```rust
/// Loads a blendable (4-channel, RGB-tintable) matcap material from disk.
///
/// Takes a name and 4 image file paths for R, G, B, K matcap channels.
/// The material becomes available immediately in the UI material selector.
///
/// Supports HDR, JPEG, PNG, EXR, and other image formats.
///
/// # Errors
/// Returns an error if the material name already exists or if any image file
/// cannot be loaded.
///
/// # Example
/// ```no_run
/// polyscope::load_blendable_material("metal", [
///     "assets/metal_r.hdr",
///     "assets/metal_g.hdr",
///     "assets/metal_b.hdr",
///     "assets/metal_k.hdr",
/// ]).unwrap();
/// ```
pub fn load_blendable_material(name: &str, filenames: [&str; 4]) -> Result<()> {
    with_context_mut(|ctx| {
        let engine = ctx.engine.as_mut().expect("engine not initialized");
        engine.load_blendable_material(name, filenames)
    })
}

/// Loads a blendable material using a base path and extension.
///
/// Automatically expands to 4 filenames by appending `_r`, `_g`, `_b`, `_k`
/// before the extension. For example:
/// `load_blendable_material_ext("metal", "assets/metal", ".hdr")`
/// loads `assets/metal_r.hdr`, `assets/metal_g.hdr`, `assets/metal_b.hdr`, `assets/metal_k.hdr`.
///
/// # Errors
/// Returns an error if the material name already exists or if any image file
/// cannot be loaded.
pub fn load_blendable_material_ext(name: &str, base: &str, ext: &str) -> Result<()> {
    let filenames = [
        format!("{base}_r{ext}"),
        format!("{base}_g{ext}"),
        format!("{base}_b{ext}"),
        format!("{base}_k{ext}"),
    ];
    load_blendable_material(name, [&filenames[0], &filenames[1], &filenames[2], &filenames[3]])
}

/// Loads a static (single-texture, non-RGB-tintable) matcap material from disk.
///
/// The same texture is used for all 4 matcap channels. Static materials
/// cannot be tinted with per-surface RGB colors.
///
/// # Errors
/// Returns an error if the material name already exists or if the image file
/// cannot be loaded.
///
/// # Example
/// ```no_run
/// polyscope::load_static_material("stone", "assets/stone.jpg").unwrap();
/// ```
pub fn load_static_material(name: &str, filename: &str) -> Result<()> {
    with_context_mut(|ctx| {
        let engine = ctx.engine.as_mut().expect("engine not initialized");
        engine.load_static_material(name, filename)
    })
}
```

**Step 2: Check how Context stores the engine**

NOTE: The `ctx.engine` pattern above assumes `Context` has a field `engine: Option<RenderEngine>`. If access is different (e.g., engine is stored separately), adjust accordingly. The existing code in `render.rs` accesses engine as `&mut self.engine` on the `App` struct, not on `Context`. Check how other public API functions that need the engine work.

If the engine is NOT on Context but only on App, the API functions will need to use a different pattern — potentially storing the engine in a separate global or passing it through Context. Investigate `with_context_mut` usage patterns for engine access.

**IMPORTANT**: The `App` struct (line 34 in `app/mod.rs`) owns the engine: `pub(super) engine: Option<RenderEngine>`. The `Context` (in polyscope-core) does NOT hold the engine. This means the public API cannot access the engine through `with_context_mut`.

**Alternative approach**: Add a global engine accessor, or store a reference/Arc in Context, or make the load functions operate on a deferred action queue that the App processes. The simplest approach: add an `engine` field to `Context` (it's already a grab-bag of global state). OR: expose the functions only on the `App` during the render loop and wire them through the UI action system.

**Recommended**: Use the UI action pattern already established in the codebase. Define a `MaterialAction` enum in `polyscope-ui`, return it from the UI panel, handle it in `render.rs` where `engine` is accessible. For the programmatic API, store a deferred-load queue in `Context` that `App::render()` drains each frame.

**Step 3: Verify it compiles**

Run: `cargo check -p polyscope`
Expected: OK

**Step 4: Commit**

```bash
git add crates/polyscope/src/lib.rs
git commit -m "feat(materials): add public load_blendable_material and load_static_material API"
```

---

### Task 6: Make material selector dynamic (UI)

**Files:**
- Modify: `crates/polyscope-ui/src/structure_ui.rs:1-21`

**Step 1: Change `build_material_selector` to accept dynamic list**

Replace the hardcoded `MATERIALS` const and update the function signature:

```rust
/// Default built-in material names (used as fallback).
const DEFAULT_MATERIALS: &[&str] = &["clay", "wax", "candy", "flat", "mud", "ceramic", "jade", "normal"];

/// Builds a material selector `ComboBox`. Returns true if the material changed.
/// `available_materials` is the list of all registered material names (built-in + custom).
/// If empty, falls back to the default built-in list.
pub fn build_material_selector(
    ui: &mut Ui,
    material: &mut String,
    available_materials: &[&str],
) -> bool {
    let materials = if available_materials.is_empty() {
        DEFAULT_MATERIALS
    } else {
        available_materials
    };

    let mut changed = false;
    egui::ComboBox::from_label("Material")
        .selected_text(material.as_str())
        .show_ui(ui, |ui| {
            for &mat in materials {
                if ui.selectable_value(material, mat.to_string(), mat).changed() {
                    changed = true;
                }
            }
        });
    changed
}
```

**Step 2: Update all call sites**

In the same file, update `build_point_cloud_ui` (line 35), `build_surface_mesh_ui` (line 82), and `build_curve_network_ui` (line 203) to pass `available_materials`:

Add `available_materials: &[&str]` parameter to each function signature, and pass it through:

```rust
if build_material_selector(ui, material, available_materials) {
```

**Step 3: Update call sites in `render.rs`**

In `crates/polyscope/src/app/render.rs`, wherever `build_point_cloud_ui`, `build_surface_mesh_ui`, or `build_curve_network_ui` is called, pass the material names from the engine:

```rust
let material_names: Vec<&str> = engine.materials.names().iter().copied().collect();
```

Then pass `&material_names` to each UI builder call.

**Step 4: Verify it compiles**

Run: `cargo check`
Expected: OK

**Step 5: Commit**

```bash
git add crates/polyscope-ui/src/structure_ui.rs crates/polyscope/src/app/render.rs
git commit -m "feat(ui): make material selector dynamic from registry"
```

---

### Task 7: Add MaterialAction enum and loading UI panel

**Files:**
- Modify: `crates/polyscope-ui/src/panels.rs`

**Step 1: Define MaterialAction enum**

Add near the top of `panels.rs` (after other action enums):

```rust
/// Actions from the material loading UI.
#[derive(Debug, Clone, PartialEq)]
pub enum MaterialAction {
    /// No action.
    None,
    /// Load a static material from a single file.
    LoadStatic { name: String, path: String },
    /// Load a blendable material from base path + extension (auto-expands _r/_g/_b/_k).
    LoadBlendable { name: String, base_path: String, extension: String },
}
```

**Step 2: Define MaterialLoadState struct**

```rust
/// UI state for the material loading panel.
#[derive(Debug, Clone, Default)]
pub struct MaterialLoadState {
    /// Material name input.
    pub name: String,
    /// File path input.
    pub path: String,
    /// Status message (success or error).
    pub status: String,
}
```

**Step 3: Build the panel function**

```rust
/// Builds the material loading section in the left panel.
/// Returns a `MaterialAction` if the user requested loading.
pub fn build_material_section(
    ui: &mut Ui,
    state: &mut MaterialLoadState,
) -> MaterialAction {
    let mut action = MaterialAction::None;

    CollapsingHeader::new("Materials")
        .default_open(false)
        .show(ui, |ui| {
            egui::Grid::new("material_load_grid")
                .num_columns(2)
                .show(ui, |ui| {
                    ui.label("Name:");
                    ui.text_edit_singleline(&mut state.name);
                    ui.end_row();

                    ui.label("File path:");
                    ui.text_edit_singleline(&mut state.path);
                    ui.end_row();
                });

            ui.horizontal(|ui| {
                if ui.button("Load Static").clicked() && !state.name.is_empty() && !state.path.is_empty() {
                    action = MaterialAction::LoadStatic {
                        name: state.name.clone(),
                        path: state.path.clone(),
                    };
                }
                if ui.button("Load Blendable").clicked() && !state.name.is_empty() && !state.path.is_empty() {
                    // Split path into base + extension for _r/_g/_b/_k expansion
                    let p = std::path::Path::new(&state.path);
                    let ext = p.extension()
                        .map(|e| format!(".{}", e.to_string_lossy()))
                        .unwrap_or_default();
                    let base = state.path.strip_suffix(&ext).unwrap_or(&state.path).to_string();
                    action = MaterialAction::LoadBlendable {
                        name: state.name.clone(),
                        base_path: base,
                        extension: ext,
                    };
                }
            });

            if !state.status.is_empty() {
                ui.label(&state.status);
            }
        });

    action
}
```

**Step 4: Export new types from polyscope-ui**

In `crates/polyscope-ui/src/lib.rs`, the `pub use panels::*;` on line 36 already re-exports everything public from `panels.rs`, so `MaterialAction` and `MaterialLoadState` will be available automatically.

**Step 5: Verify it compiles**

Run: `cargo check -p polyscope-ui`
Expected: OK

**Step 6: Commit**

```bash
git add crates/polyscope-ui/src/panels.rs
git commit -m "feat(ui): add material loading panel with MaterialAction enum"
```

---

### Task 8: Wire material loading into App render loop

**Files:**
- Modify: `crates/polyscope/src/app/mod.rs` (add field)
- Modify: `crates/polyscope/src/app/render.rs` (wire up panel and handle action)

**Step 1: Add `material_load_state` field to App**

In `app/mod.rs`, add after `tone_mapping_settings` (line 81):

```rust
    // Material loading UI state
    pub(super) material_load_state: polyscope_ui::MaterialLoadState,
```

And initialize it in the App constructor as `material_load_state: polyscope_ui::MaterialLoadState::default(),`.

**Step 2: Add material panel to left panel in render.rs**

In `render.rs`, after the appearance section (after line 945) and before tone mapping (line 948), add:

```rust
            // Material loading section
            let material_action = polyscope_ui::build_material_section(
                ui,
                &mut self.material_load_state,
            );
            match material_action {
                polyscope_ui::MaterialAction::LoadStatic { name, path } => {
                    match engine.load_static_material(&name, &path) {
                        Ok(()) => {
                            self.material_load_state.status = format!("Loaded static material '{name}'");
                        }
                        Err(e) => {
                            self.material_load_state.status = format!("Error: {e}");
                        }
                    }
                }
                polyscope_ui::MaterialAction::LoadBlendable { name, base_path, extension } => {
                    let filenames = [
                        format!("{base_path}_r{extension}"),
                        format!("{base_path}_g{extension}"),
                        format!("{base_path}_b{extension}"),
                        format!("{base_path}_k{extension}"),
                    ];
                    match engine.load_blendable_material(
                        &name,
                        [&filenames[0], &filenames[1], &filenames[2], &filenames[3]],
                    ) {
                        Ok(()) => {
                            self.material_load_state.status =
                                format!("Loaded blendable material '{name}'");
                        }
                        Err(e) => {
                            self.material_load_state.status = format!("Error: {e}");
                        }
                    }
                }
                polyscope_ui::MaterialAction::None => {}
            }
```

**Step 3: Verify it compiles**

Run: `cargo check`
Expected: OK

**Step 4: Commit**

```bash
git add crates/polyscope/src/app/mod.rs crates/polyscope/src/app/render.rs
git commit -m "feat(materials): wire material loading UI into App render loop"
```

---

### Task 9: Add public API via deferred queue on Context

**Files:**
- Modify: `crates/polyscope-core/src/state.rs` (add deferred material load queue)
- Modify: `crates/polyscope/src/lib.rs` (add public API functions)
- Modify: `crates/polyscope/src/app/render.rs` (drain queue each frame)

**Step 1: Add deferred load queue to Context**

In `state.rs`, add to the `Context` struct:

```rust
    /// Deferred material load requests (processed by App each frame).
    pub material_load_queue: Vec<MaterialLoadRequest>,
```

And define:

```rust
/// A deferred request to load a material from disk.
#[derive(Debug, Clone)]
pub enum MaterialLoadRequest {
    /// Load a static material.
    Static { name: String, path: String },
    /// Load a blendable material from 4 files.
    Blendable { name: String, filenames: [String; 4] },
}
```

**Step 2: Add public API in lib.rs**

```rust
pub fn load_blendable_material(name: &str, filenames: [&str; 4]) -> Result<()> {
    with_context_mut(|ctx| {
        ctx.material_load_queue.push(
            polyscope_core::state::MaterialLoadRequest::Blendable {
                name: name.to_string(),
                filenames: [
                    filenames[0].to_string(),
                    filenames[1].to_string(),
                    filenames[2].to_string(),
                    filenames[3].to_string(),
                ],
            },
        );
    });
    Ok(())
}

pub fn load_blendable_material_ext(name: &str, base: &str, ext: &str) -> Result<()> {
    load_blendable_material(name, [
        &format!("{base}_r{ext}"),
        &format!("{base}_g{ext}"),
        &format!("{base}_b{ext}"),
        &format!("{base}_k{ext}"),
    ])
}

pub fn load_static_material(name: &str, filename: &str) -> Result<()> {
    with_context_mut(|ctx| {
        ctx.material_load_queue.push(
            polyscope_core::state::MaterialLoadRequest::Static {
                name: name.to_string(),
                path: filename.to_string(),
            },
        );
    });
    Ok(())
}
```

**Step 3: Drain queue in render.rs**

At the start of `render()`, before the UI code, drain the queue:

```rust
        // Process deferred material loads
        let load_requests: Vec<_> = crate::with_context_mut(|ctx| {
            std::mem::take(&mut ctx.material_load_queue)
        });
        for req in load_requests {
            match req {
                polyscope_core::state::MaterialLoadRequest::Static { name, path } => {
                    if let Err(e) = engine.load_static_material(&name, &path) {
                        eprintln!("Failed to load static material '{name}': {e}");
                    }
                }
                polyscope_core::state::MaterialLoadRequest::Blendable { name, filenames } => {
                    if let Err(e) = engine.load_blendable_material(
                        &name,
                        [&filenames[0], &filenames[1], &filenames[2], &filenames[3]],
                    ) {
                        eprintln!("Failed to load blendable material '{name}': {e}");
                    }
                }
            }
        }
```

**Step 4: Verify it compiles**

Run: `cargo check`
Expected: OK

**Step 5: Commit**

```bash
git add crates/polyscope-core/src/state.rs crates/polyscope/src/lib.rs crates/polyscope/src/app/render.rs
git commit -m "feat(materials): add public load_blendable/static_material API via deferred queue"
```

---

### Task 10: Add unit tests for MaterialRegistry

**Files:**
- Modify: `crates/polyscope-render/src/materials.rs` (add tests to existing `mod tests`)

**Step 1: Add tests**

```rust
    #[test]
    fn test_material_registry_has() {
        let registry = MaterialRegistry::new();
        assert!(registry.has("clay"));
        assert!(registry.has("mud"));
        assert!(!registry.has("nonexistent"));
    }

    #[test]
    fn test_material_registry_names_order() {
        let mut registry = MaterialRegistry::new();
        // Register custom materials
        registry.register(Material::new("zebra"));
        registry.register(Material::new("alpha"));

        let names = registry.names();
        // Built-ins come first in canonical order
        assert_eq!(names[0], "clay");
        assert_eq!(names[1], "wax");
        assert_eq!(names[7], "normal");
        // Custom materials after built-ins, sorted alphabetically
        assert_eq!(names[8], "alpha");
        assert_eq!(names[9], "zebra");
    }

    #[test]
    fn test_material_registry_custom() {
        let mut registry = MaterialRegistry::new();
        registry.register(Material::new("custom_mat"));
        assert!(registry.has("custom_mat"));
        assert!(registry.get("custom_mat").is_some());
    }
```

**Step 2: Run tests**

Run: `cargo test -p polyscope-render -- materials`
Expected: All tests pass

**Step 3: Commit**

```bash
git add crates/polyscope-render/src/materials.rs
git commit -m "test(materials): add unit tests for MaterialRegistry has(), names() ordering, custom registration"
```

---

### Task 11: Update todo.md

**Files:**
- Modify: `todo.md`

**Step 1: Mark custom material loading as complete**

Change the line:
```
- [ ] **Custom Material Loading** - User-provided matcap textures (`loadBlendableMaterial` / `loadStaticMaterial`)
```
to:
```
- [x] **Custom Material Loading** - User-provided matcap textures (`load_blendable_material` / `load_static_material`)
```

**Step 2: Commit**

```bash
git add todo.md
git commit -m "docs: mark custom material loading as complete in todo.md"
```

---

### Task 12: Build and verify end-to-end

**Step 1: Full build**

Run: `cargo build`
Expected: Clean build, no warnings

**Step 2: Run clippy**

Run: `cargo clippy`
Expected: No new warnings

**Step 3: Run all tests**

Run: `cargo test`
Expected: All tests pass

**Step 4: Final commit (if any fixups needed)**

```bash
git add -A
git commit -m "chore: fix any build warnings from custom material loading"
```
