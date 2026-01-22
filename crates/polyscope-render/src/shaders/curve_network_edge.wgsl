// Curve network edge shader for line rendering
// Renders edges as simple lines with per-edge colors

struct CameraUniforms {
    view: mat4x4<f32>,
    proj: mat4x4<f32>,
    view_proj: mat4x4<f32>,
    inv_proj: mat4x4<f32>,
    camera_pos: vec3<f32>,
    _padding: f32,
}

struct CurveNetworkUniforms {
    color: vec4<f32>,         // Base color (RGBA)
    radius: f32,              // Line thickness (for future tube rendering)
    radius_is_relative: u32,  // Whether radius is relative to scene scale
    render_mode: u32,         // 0 = line, 1 = tube
    _padding: f32,
}

@group(0) @binding(0) var<uniform> camera: CameraUniforms;
@group(0) @binding(1) var<uniform> cn_uniforms: CurveNetworkUniforms;
@group(0) @binding(2) var<storage, read> node_positions: array<vec4<f32>>;
@group(0) @binding(3) var<storage, read> node_colors: array<vec4<f32>>;
@group(0) @binding(4) var<storage, read> edge_vertices: array<vec4<f32>>;
@group(0) @binding(5) var<storage, read> edge_colors: array<vec4<f32>>;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) edge_color: vec4<f32>,
}

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_index: u32,
) -> VertexOutput {
    var out: VertexOutput;

    // Read edge vertex position
    // Edge vertices are stored as [tail0, tip0, tail1, tip1, ...]
    let position = edge_vertices[vertex_index].xyz;

    // Get edge index (each edge has 2 vertices)
    let edge_index = vertex_index / 2u;

    // Transform position
    out.clip_position = camera.view_proj * vec4<f32>(position, 1.0);
    out.world_position = position;

    // Get color - use edge color if non-zero, otherwise base color
    let ec = edge_colors[edge_index];
    let color_sum = ec.r + ec.g + ec.b;
    if (color_sum > 0.001) {
        out.edge_color = ec;
    } else {
        out.edge_color = cn_uniforms.color;
    }

    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Simple unlit color for lines
    // Lines don't have normals, so we can't do proper lighting
    // Just return the edge color with slight ambient darkening
    let ambient_factor = 0.8;
    let color = in.edge_color.rgb * ambient_factor;

    return vec4<f32>(color, in.edge_color.a);
}
