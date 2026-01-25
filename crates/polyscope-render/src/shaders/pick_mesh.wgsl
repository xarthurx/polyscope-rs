// Pick shader for surface meshes - outputs encoded face index
//
// Each face is rendered with a unique color encoding the structure ID and face index.
// Uses flat interpolation to ensure the entire face gets the same color.

struct CameraUniforms {
    view: mat4x4<f32>,
    proj: mat4x4<f32>,
    view_proj: mat4x4<f32>,
    inv_proj: mat4x4<f32>,
    camera_pos: vec3<f32>,
    _padding: f32,
}

struct PickUniforms {
    structure_id: u32,
    _padding: vec3<f32>,
}

@group(0) @binding(0) var<uniform> camera: CameraUniforms;
@group(0) @binding(1) var<uniform> pick: PickUniforms;
@group(0) @binding(2) var<storage, read> positions: array<vec4<f32>>;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) @interpolate(flat) face_index: u32,
}

// Encode structure_id (12 bits) and element_id (12 bits) into RGB
fn encode_pick_id(structure_id: u32, element_id: u32) -> vec3<f32> {
    let s = structure_id & 0xFFFu;
    let e = element_id & 0xFFFu;
    let r = f32(s >> 4u) / 255.0;
    let g = f32(((s & 0xFu) << 4u) | (e >> 8u)) / 255.0;
    let b = f32(e & 0xFFu) / 255.0;
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

    // Face index = vertex_index / 3 (each face has 3 vertices)
    out.face_index = vertex_index / 3u;

    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let color = encode_pick_id(pick.structure_id, in.face_index);
    return vec4<f32>(color, 1.0);
}
