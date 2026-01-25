# GPU Picking Design

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Replace screen-space approximation picking with pixel-perfect GPU picking that identifies exact elements (point #42, face #127, etc.)

**Architecture:** Render all structures to an offscreen pick buffer with encoded color IDs, then read back the pixel at click position and decode to identify structure + element.

**Tech Stack:** wgpu (pick texture, staging buffer, render pipelines), WGSL shaders

---

## Overview

The current picking uses screen-space projection with a 20px threshold, which is inaccurate for sparse point clouds and overlapping structures. GPU picking renders each element with a unique color encoding its ID, providing pixel-perfect accuracy.

## Pick Buffer Infrastructure

```
┌─────────────────────────────────────────────────────────┐
│                    RenderEngine                          │
│                                                          │
│  ┌──────────────┐  ┌──────────────┐  ┌───────────────┐  │
│  │ Pick Texture │  │ Pick Depth   │  │ Staging Buffer│  │
│  │ RGBA8Unorm   │  │ Depth24Plus  │  │ (CPU readable)│  │
│  │ same as view │  │ same as view │  │ 4 bytes       │  │
│  └──────────────┘  └──────────────┘  └───────────────┘  │
│         │                 │                  ▲          │
│         ▼                 ▼                  │          │
│  ┌─────────────────────────────────┐        │          │
│  │       Pick Render Pass          │────────┘          │
│  │  (renders all structures with   │   copy single     │
│  │   encoded color IDs)            │   pixel on click  │
│  └─────────────────────────────────┘                   │
└─────────────────────────────────────────────────────────┘
```

**Key resources:**
- **Pick texture**: RGBA8Unorm (not sRGB) for exact color values, same size as viewport
- **Pick depth texture**: Depth24Plus for proper occlusion
- **Staging buffer**: 4 bytes, MAP_READ usage for CPU readback
- Pick pass only runs on click, not every frame

## ID Encoding Scheme

24-bit RGB encoding with 12 bits for structure ID and 12 bits for element ID:

```
        R (8 bits)      G (8 bits)      B (8 bits)
       ┌───────────┐   ┌───────────┐   ┌───────────┐
       │ struct_hi │   │ struct_lo │   │ element_lo│
       │  (8 bits) │   │ elem_hi   │   │  (8 bits) │
       └───────────┘   └───────────┘   └───────────┘

       Structure ID (12 bits)    Element ID (12 bits)
       ├──────────────────┤     ├──────────────────┤
       R[7:0] + G[7:4]          G[3:0] + B[7:0]
```

**Encoding:**
```rust
fn encode_pick_id(structure_id: u16, element_id: u16) -> [u8; 3] {
    let s = structure_id & 0xFFF;  // 12 bits
    let e = element_id & 0xFFF;    // 12 bits
    [
        (s >> 4) as u8,                        // R: struct bits 11-4
        ((s & 0xF) << 4 | (e >> 8)) as u8,     // G: struct bits 3-0 + elem bits 11-8
        (e & 0xFF) as u8,                      // B: elem bits 7-0
    ]
}
```

**Decoding:**
```rust
fn decode_pick_id(r: u8, g: u8, b: u8) -> (u16, u16) {
    let structure_id = ((r as u16) << 4) | ((g as u16) >> 4);
    let element_id = (((g & 0xF) as u16) << 8) | (b as u16);
    (structure_id, element_id)
}
```

**Special values:**
- `(0, 0)` = background (nothing clicked)
- Structure IDs start at 1

## Pick Shaders

Each structure type needs a pick shader that outputs encoded color.

**Shared uniform:**
```wgsl
struct PickUniforms {
    structure_id: u32,    // 12-bit structure ID
    base_element: u32,    // Starting element index
    _padding: vec2<f32>,
}

fn encode_pick_id(structure_id: u32, element_id: u32) -> vec3<f32> {
    let s = structure_id & 0xFFFu;
    let e = element_id & 0xFFFu;
    let r = f32(s >> 4u) / 255.0;
    let g = f32(((s & 0xFu) << 4u) | (e >> 8u)) / 255.0;
    let b = f32(e & 0xFFu) / 255.0;
    return vec3<f32>(r, g, b);
}
```

**Shaders needed:**
- `pick.wgsl` - PointCloud (exists, needs integration)
- `pick_mesh.wgsl` - SurfaceMesh (face index via flat interpolation)
- `pick_curve.wgsl` - CurveNetwork (edge index)
- `pick_volume.wgsl` - VolumeMesh (cell index)
- `pick_camera.wgsl` - CameraView (structure only)

## RenderEngine Additions

```rust
pub struct RenderEngine {
    // ... existing fields ...

    // Pick system
    pick_texture: Option<wgpu::Texture>,
    pick_texture_view: Option<wgpu::TextureView>,
    pick_depth_texture: Option<wgpu::Texture>,
    pick_depth_view: Option<wgpu::TextureView>,
    pick_staging_buffer: Option<wgpu::Buffer>,
    pick_pipelines: HashMap<String, wgpu::RenderPipeline>,
    structure_id_map: HashMap<(String, String), u16>,
    structure_id_reverse: HashMap<u16, (String, String)>,
    next_structure_id: u16,
}

impl RenderEngine {
    pub fn init_pick_system(&mut self) { ... }
    pub fn assign_structure_id(&mut self, type_name: &str, name: &str) -> u16 { ... }
    pub fn remove_structure_id(&mut self, type_name: &str, name: &str) { ... }
    pub fn render_pick_pass(&mut self, structures: &[&dyn Structure]) { ... }
    pub fn pick_at(&self, x: u32, y: u32) -> Option<PickResult> { ... }
}
```

## Pick Flow

```
User clicks at (x, y)
        │
        ▼
┌─────────────────────────┐
│  render_pick_pass()     │  ← Render all structures to pick buffer
└───────────┬─────────────┘
            ▼
┌─────────────────────────┐
│  Copy pixel (x,y) to    │  ← encoder.copy_texture_to_buffer()
│  staging buffer         │
└───────────┬─────────────┘
            ▼
┌─────────────────────────┐
│  Map staging buffer     │  ← buffer.slice(..).map_async()
│  Read RGBA bytes        │
└───────────┬─────────────┘
            ▼
┌─────────────────────────┐
│  Decode to PickResult   │  ← decode_pick_id(r, g, b)
│  Lookup structure name  │
└─────────────────────────┘
```

## App Integration

Replace `pick_structure_at_screen_pos()` with GPU picking:

```rust
fn pick_at(&mut self, click_pos: Vec2, engine: &mut RenderEngine) -> Option<PickResult> {
    engine.render_pick_pass(&self.get_visible_structures());
    let x = click_pos.x as u32;
    let y = click_pos.y as u32;
    engine.pick_at(x, y)
}
```

## Updated PickResult

```rust
pub struct PickResult {
    pub hit: bool,
    pub structure_type: String,
    pub structure_name: String,
    pub element_type: PickElementType,  // Point, Vertex, Face, Edge, Cell
    pub element_index: u32,
    pub screen_pos: Vec2,
}
```

## Future Extensions

After this implementation, can add:
- Hover highlighting (pick on mouse move, highlight hovered element)
- Selection highlighting (outline or glow on selected element)
- Element info panel (show position, scalar value, etc.)
