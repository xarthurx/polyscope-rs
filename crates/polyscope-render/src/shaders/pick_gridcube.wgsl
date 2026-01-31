// Pick shader for volume grid gridcube instances
//
// Each gridcube instance (node or cell) is rendered with a unique color
// encoding its global index. Uses the same vertex positioning logic as
// gridcube.wgsl but outputs a flat pick color instead of shaded appearance.

struct CameraUniforms {
    view: mat4x4<f32>,
    proj: mat4x4<f32>,
    view_proj: mat4x4<f32>,
    inv_proj: mat4x4<f32>,
    camera_pos: vec3<f32>,
    _padding: f32,
}

struct GridcubePickUniforms {
    model: mat4x4<f32>,
    global_start: u32,
    cube_size_factor: f32,
    _pad0: f32,
    _pad1: f32,
}

@group(0) @binding(0) var<uniform> camera: CameraUniforms;
@group(0) @binding(1) var<uniform> pick: GridcubePickUniforms;
@group(0) @binding(2) var<storage, read> cube_positions: array<vec4<f32>>;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) @interpolate(flat) instance_id: u32,
}

// Unit cube has 36 vertices (12 triangles, 6 faces)
const VERTS_PER_CUBE: u32 = 36u;

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

    let instance_id = vertex_index / VERTS_PER_CUBE;
    let local_vertex_id = vertex_index % VERTS_PER_CUBE;

    // Read cube template position from storage buffer
    let local_pos = cube_positions[local_vertex_id].xyz;

    // Read instance data: center position and spacing
    let instance_data = cube_positions[VERTS_PER_CUBE + instance_id];
    let center = instance_data.xyz;
    let half_size = instance_data.w * pick.cube_size_factor;

    // Scale and translate the unit cube to instance position
    let scaled_pos = local_pos * half_size + center;

    // Apply model transform
    let world_position = (pick.model * vec4<f32>(scaled_pos, 1.0)).xyz;

    out.clip_position = camera.view_proj * vec4<f32>(world_position, 1.0);
    out.instance_id = instance_id;

    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let global_index = pick.global_start + in.instance_id;
    let color = index_to_color(global_index);
    return vec4<f32>(color, 1.0);
}
