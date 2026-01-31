// Pick shader for surface meshes - outputs encoded face index
//
// Each face is rendered with a unique color encoding its global index.
// Uses a face_indices storage buffer to map GPU triangle indices to polygon face indices.
// Uses flat interpolation to ensure the entire triangle gets the same color.

struct CameraUniforms {
    view: mat4x4<f32>,
    proj: mat4x4<f32>,
    view_proj: mat4x4<f32>,
    inv_proj: mat4x4<f32>,
    camera_pos: vec3<f32>,
    _padding: f32,
}

struct MeshPickUniforms {
    global_start: u32,
    _padding0: f32,
    _padding1: f32,
    _padding2: f32,
    model: mat4x4<f32>,
}

@group(0) @binding(0) var<uniform> camera: CameraUniforms;
@group(0) @binding(1) var<uniform> pick: MeshPickUniforms;
@group(0) @binding(2) var<storage, read> positions: array<vec4<f32>>;
@group(0) @binding(3) var<storage, read> face_indices: array<u32>;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) @interpolate(flat) tri_index: u32,
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

    // Get vertex position from storage buffer and apply model transform
    let local_pos = positions[vertex_index].xyz;
    let world_pos = (pick.model * vec4<f32>(local_pos, 1.0)).xyz;
    out.clip_position = camera.view_proj * vec4<f32>(world_pos, 1.0);

    // Triangle index = vertex_index / 3 (each triangle has 3 vertices in expanded buffer)
    out.tri_index = vertex_index / 3u;

    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Look up the polygon face index for this GPU triangle
    let face_index = face_indices[in.tri_index];
    let global_index = pick.global_start + face_index;
    let color = index_to_color(global_index);
    return vec4<f32>(color, 1.0);
}
