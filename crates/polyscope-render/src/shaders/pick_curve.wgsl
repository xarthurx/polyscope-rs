// Pick shader for curve networks - outputs encoded edge index
//
// Each edge is rendered with a unique color encoding its global index.
// Uses flat interpolation to ensure the entire edge gets the same color.

struct CameraUniforms {
    view: mat4x4<f32>,
    proj: mat4x4<f32>,
    view_proj: mat4x4<f32>,
    inv_proj: mat4x4<f32>,
    camera_pos: vec3<f32>,
    _padding: f32,
}

struct PickUniforms {
    global_start: u32,
    line_width: f32,
    _padding: vec2<f32>,
}

@group(0) @binding(0) var<uniform> camera: CameraUniforms;
@group(0) @binding(1) var<uniform> pick: PickUniforms;
@group(0) @binding(2) var<storage, read> positions: array<vec4<f32>>;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) @interpolate(flat) edge_index: u32,
}

// Encode a flat 24-bit global index into RGB color
fn index_to_color(index: u32) -> vec3<f32> {
    let r = f32((index >> 16u) & 0xFFu) / 255.0;
    let g = f32((index >> 8u) & 0xFFu) / 255.0;
    let b = f32(index & 0xFFu) / 255.0;
    return vec3<f32>(r, g, b);
}

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_index: u32,
) -> VertexOutput {
    var out: VertexOutput;

    // Get vertex position from storage buffer
    let world_pos = positions[vertex_index].xyz;
    out.clip_position = camera.view_proj * vec4<f32>(world_pos, 1.0);

    // Edge index = vertex_index / 2 (each edge has 2 vertices for lines)
    out.edge_index = vertex_index / 2u;

    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let color = index_to_color(pick.global_start + in.edge_index);
    return vec4<f32>(color, 1.0);
}
