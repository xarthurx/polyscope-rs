# Matcap Material System Design

## Overview

Port the C++ Polyscope matcap material rendering system to polyscope-rs. This replaces the current hardcoded parametric lighting (fixed ambient + diffuse) with texture-based matcap lighting that matches the C++ visual output exactly.

## Decisions

- **4-channel blend**: Faithful C++ port with R/G/B/K texture channels for color tinting
- **Texture embedding**: `include_bytes!` on HDR files extracted from C++ bindata
- **Scope**: All three structure types (SurfaceMesh, PointCloud, CurveNetwork)
- **Texture source**: Decode from C++ `bindata_*.cpp` binary arrays

## Data Layer

### Texture Storage

Matcap HDR files extracted from C++ `bindata_*.cpp` live in `crates/polyscope-render/data/matcaps/`:

```
data/matcaps/
  clay_r.hdr, clay_g.hdr, clay_b.hdr, clay_k.hdr      (blendable)
  wax_r.hdr, wax_g.hdr, wax_b.hdr, wax_k.hdr          (blendable)
  candy_r.hdr, candy_g.hdr, candy_b.hdr, candy_k.hdr   (blendable)
  flat_r.hdr, flat_g.hdr, flat_b.hdr, flat_k.hdr       (blendable)
  mud.hdr, ceramic.hdr, jade.hdr, normal.hdr            (static)
```

Blendable materials have 4 separate HDR files. Static materials have 1 HDR file reused for all 4 channels. The textures are original creations by Nicholas Sharp, rendered in Blender 2.81 as RADIANCE HDR format, MIT licensed.

### Embedding & Decoding

Each HDR file is embedded via `include_bytes!("data/matcaps/clay_r.hdr")`. At engine init, the `image` crate (already in workspace at v0.25) decodes RADIANCE HDR → float RGB → uploaded as `wgpu::TextureFormat::Rgba16Float` textures (pad RGB→RGBA with alpha=1.0). Linear filtering sampler.

### Material Struct Enhancement

`materials.rs` gains `is_blendable: bool` on `Material`. A new struct:

```rust
pub struct MatcapTextureSet {
    pub tex_r: wgpu::TextureView,
    pub tex_g: wgpu::TextureView,
    pub tex_b: wgpu::TextureView,
    pub tex_k: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
    pub bind_group: wgpu::BindGroup,
}
```

Stored in `HashMap<String, MatcapTextureSet>` on `RenderEngine`. Static materials point all 4 views at the same texture. Created once at engine init.

## Shader Integration

### Matcap Lighting Function

New WGSL function (shared across all shaders):

```wgsl
fn light_surface_matcap(
    normal: vec3<f32>, color: vec3<f32>,
    t_r: texture_2d<f32>, t_g: texture_2d<f32>,
    t_b: texture_2d<f32>, t_k: texture_2d<f32>,
    s: sampler
) -> vec3<f32> {
    var n = normalize(normal);
    n.y = -n.y;
    n = n * 0.98;
    let uv = n.xy * 0.5 + vec2(0.5);
    let mat_r = textureSample(t_r, s, uv).rgb;
    let mat_g = textureSample(t_g, s, uv).rgb;
    let mat_b = textureSample(t_b, s, uv).rgb;
    let mat_k = textureSample(t_k, s, uv).rgb;
    return color.r * mat_r + color.g * mat_g
         + color.b * mat_b + (1.0 - color.r - color.g - color.b) * mat_k;
}
```

Key details:
- View-space normal → UV: map normal.xy from [-1,1] to [0,1]
- Y flip for camera convention
- 0.98 scale to avoid edge artifacts
- 4-channel weighted blend with K as remainder

### Bind Group Layout

New bind group at **Group 2** for matcap textures:
- Binding 0: `texture_2d<f32>` (mat_r)
- Binding 1: `texture_2d<f32>` (mat_g)
- Binding 2: `texture_2d<f32>` (mat_b)
- Binding 3: `texture_2d<f32>` (mat_k)
- Binding 4: `sampler` (linear filtering)

Current layout preserved:
- Group 0: Per-object (camera + object uniforms + buffers)
- Group 1: Slice planes
- Group 2: Matcap textures (NEW)

### Flat Material Handling

The `is_flat` flag is communicated via a uniform (already in MeshUniforms etc.). When `is_flat == 1`, shader skips matcap and uses `color` directly. No separate pipeline needed.

## Pipeline Wiring

### Engine Init

1. Decode all embedded HDR bytes → float pixel data
2. Upload each as `Rgba16Float` texture
3. Create shared linear `Sampler`
4. Create `matcap_bind_group_layout` (5 entries)
5. Pre-create one `BindGroup` per material (8 bind groups)
6. Store in `HashMap<String, MatcapTextureSet>` on engine

### Pipeline Layout Changes

All three render pipelines (mesh, point, vector) and their reflected variants add `matcap_bind_group_layout` at Group 2.

### Per-Frame Render

When drawing a structure, bind its material's pre-built bind group at Group 2. Material name flows: structure field → render data → engine lookup → bind group.

## Structure Integration

### New Fields & Methods

**SurfaceMesh** and **PointCloud** gain:
- `material: String` field (default: `"clay"`)
- `set_material(name: &str)` / `material()` methods

**CurveNetwork** already has the field — just needs render pipeline integration.

### UI

Material dropdown extended to SurfaceMesh and PointCloud in the egui structure panel. Lists all registered material names.

## Extraction Script

One-time Python script `scripts/extract_matcaps.py`:
1. Reads each `bindata_*.cpp` from C++ source
2. Parses the byte arrays
3. Writes raw bytes to `.hdr` files in `data/matcaps/`
4. Not part of the build system

## Affected Files

### New Files
- `crates/polyscope-render/data/matcaps/*.hdr` (texture data)
- `crates/polyscope-render/src/shaders/matcap_light.wgsl` (shared function)
- `scripts/extract_matcaps.py` (one-time extraction)

### Modified Files
- `crates/polyscope-render/src/materials.rs` — Add `is_blendable`, `MatcapTextureSet`, loading logic
- `crates/polyscope-render/src/engine/mod.rs` — Store matcap textures, bind group layout, init loading
- `crates/polyscope-render/src/engine/pipelines.rs` — Add Group 2 to pipeline layouts
- `crates/polyscope-render/src/shaders/surface_mesh.wgsl` — Replace hardcoded lighting with matcap
- `crates/polyscope-render/src/shaders/point_sphere.wgsl` — Same
- `crates/polyscope-render/src/shaders/curve_network_edge.wgsl` — Same
- `crates/polyscope-render/src/shaders/curve_network_tube_render.wgsl` — Same
- `crates/polyscope/src/app.rs` — Bind matcap group in render passes
- `crates/polyscope-structures/src/surface_mesh/mod.rs` — Add material field
- `crates/polyscope-structures/src/point_cloud/mod.rs` — Add material field
- `crates/polyscope-ui/src/panels.rs` — Material dropdown for mesh/point
