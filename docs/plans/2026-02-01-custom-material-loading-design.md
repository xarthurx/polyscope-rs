# Custom Material Loading Design

## Overview

Add support for loading user-provided matcap textures at runtime, matching C++ Polyscope's `loadBlendableMaterial()` and `loadStaticMaterial()` API. Includes both programmatic API and a UI panel for loading materials from file.

## Public API

```rust
// In polyscope crate (public API)

/// Load a 4-channel blendable material from disk.
/// Files should be HDR/EXR/PNG/JPEG images for R, G, B, K channels.
/// Blendable materials support RGB tinting via the 4-channel blend formula.
pub fn load_blendable_material(name: &str, filenames: [&str; 4]) -> Result<()>;

/// Convenience: auto-expands base + ext into 4 filenames.
/// e.g., ("custom", "/path/to/custom", ".hdr") → [custom_r.hdr, custom_g.hdr, custom_b.hdr, custom_k.hdr]
pub fn load_blendable_material_ext(name: &str, base: &str, ext: &str) -> Result<()>;

/// Load a single-texture static material from disk.
/// Same texture used for all 4 channels (not RGB-tintable).
pub fn load_static_material(name: &str, filename: &str) -> Result<()>;
```

**Usage:**
```rust
use polyscope::*;

fn main() -> Result<()> {
    init()?;

    // Load custom blendable material (4 HDR files)
    load_blendable_material("my_metal", [
        "assets/metal_r.hdr",
        "assets/metal_g.hdr",
        "assets/metal_b.hdr",
        "assets/metal_k.hdr",
    ])?;

    // Load custom static material (1 file)
    load_static_material("my_stone", "assets/stone.jpg")?;

    // Use on a structure
    let mesh = register_surface_mesh("bunny", vertices, faces);
    mesh.set_material("my_metal");

    show();
    Ok(())
}
```

Can be called after `init()`. Materials become available immediately in UI combo boxes.

## Internal Changes

### 1. polyscope-render: MaterialRegistry

**File:** `crates/polyscope-render/src/materials.rs`

- Add `MaterialRegistry::register(name: &str, material: Material)` — inserts a new material, returns error if name already exists.
- Add `MaterialRegistry::names() -> Vec<String>` — returns all registered material names (built-in + custom), sorted with built-ins first.
- Existing `init_matcap_textures()` unchanged for built-in materials.

### 2. polyscope-render: RenderEngine loading methods

**File:** `crates/polyscope-render/src/engine/mod.rs`

Add methods on `RenderEngine`:

```rust
impl RenderEngine {
    /// Load a blendable material from 4 image files on disk.
    pub fn load_blendable_material(&mut self, name: &str, filenames: [&str; 4]) -> Result<()> {
        // 1. Check duplicate name
        // 2. For each file: image::open(path) → DynamicImage → to_rgb32f()
        // 3. Pad RGB→RGBA, upload via upload_matcap_texture() (f32→f16, Rgba16Float)
        // 4. Create MatcapTextureSet with 4 distinct views + bind group
        // 5. Insert into self.matcap_textures
        // 6. Register Material in self.material_registry (is_blendable=true)
    }

    /// Load a static material from 1 image file on disk.
    pub fn load_static_material(&mut self, name: &str, filename: &str) -> Result<()> {
        // 1. Check duplicate name
        // 2. image::open(path) → DynamicImage → to_rgb32f()
        // 3. Upload single texture
        // 4. Create MatcapTextureSet with same view in all 4 slots + bind group
        // 5. Insert into self.matcap_textures
        // 6. Register Material in self.material_registry (is_blendable=false)
    }
}
```

**Refactoring needed:** Extract common GPU upload logic from `init_matcap_textures()` so both embedded and file-based paths share the same texture creation code. The existing `decode_matcap_image()` takes `&[u8]` bytes; add a `decode_matcap_image_from_file(path)` variant that calls `image::open()` then feeds into the same pipeline.

### 3. polyscope: Public API wrappers

**File:** `crates/polyscope/src/lib.rs` (or appropriate submodule)

Thin wrappers that acquire the global context and delegate to `RenderEngine`:

```rust
pub fn load_blendable_material(name: &str, filenames: [&str; 4]) -> Result<()> {
    with_context_mut(|ctx| {
        ctx.engine.load_blendable_material(name, filenames)
    })
}

pub fn load_static_material(name: &str, filename: &str) -> Result<()> {
    with_context_mut(|ctx| {
        ctx.engine.load_static_material(name, filename)
    })
}

pub fn load_blendable_material_ext(name: &str, base: &str, ext: &str) -> Result<()> {
    let filenames = [
        format!("{}_r{}", base, ext),
        format!("{}_g{}", base, ext),
        format!("{}_b{}", base, ext),
        format!("{}_k{}", base, ext),
    ];
    load_blendable_material(name, [&filenames[0], &filenames[1], &filenames[2], &filenames[3]])
}
```

### 4. polyscope-ui: Dynamic material list

**File:** `crates/polyscope-ui/src/structure_ui.rs`

- Remove hardcoded `const MATERIALS: &[&str] = &["clay", "wax", ...]`
- `build_material_selector()` takes a `&[String]` list of material names from `MaterialRegistry::names()`
- Custom materials appear after built-ins in combo box

### 5. polyscope-ui: Material loading panel

**File:** `crates/polyscope-ui/src/structure_ui.rs` (or new section in main UI)

Add a "Load Material" collapsing section in the left panel:

- Text input: "Material name"
- Text input: "File path" (base path for blendable, full path for static)
- Button: "Load Static Material"
- Button: "Load Blendable Material" (auto-expands _r/_g/_b/_k suffixes)
- Status label: shows success or error message

UI actions dispatch to `RenderEngine::load_blendable_material()` / `load_static_material()` through context.

## No shader changes

The existing `light_surface_matcap()` WGSL function and Group 2 bind group layout handle custom materials identically to built-ins. No shader modifications needed.

## Error handling

- Duplicate material name → `Err("material 'name' already exists")`
- File not found → `Err("failed to open 'path': ...")`
- Image decode failure → `Err("failed to decode image 'path': ...")`
- Zero-size image → `Err("image 'path' has zero dimensions")`
- All errors use the existing `polyscope_core::Result` type.

## Files touched

| File | Change |
|------|--------|
| `crates/polyscope-render/src/materials.rs` | `register()`, `names()`, `decode_from_file()`, refactor upload |
| `crates/polyscope-render/src/engine/mod.rs` | `load_blendable_material()`, `load_static_material()` |
| `crates/polyscope/src/lib.rs` | Public API wrappers + re-exports |
| `crates/polyscope-ui/src/structure_ui.rs` | Dynamic material list, loading panel UI |
