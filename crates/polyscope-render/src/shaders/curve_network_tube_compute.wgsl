// Compute shader for generating cylinder bounding box geometry
// Each edge gets a bounding box (36 vertices = 12 triangles)

struct CurveNetworkUniforms {
    color: vec4<f32>,
    radius: f32,
    radius_is_relative: u32,
    render_mode: u32,
    _padding: f32,
}

struct GeneratedVertex {
    position: vec4<f32>,
    edge_id_and_vertex_id: vec4<u32>,  // edge_id, vertex_id, padding, padding
}

@group(0) @binding(0) var<storage, read> edge_vertices: array<vec4<f32>>;
@group(0) @binding(1) var<uniform> uniforms: CurveNetworkUniforms;
@group(0) @binding(2) var<storage, read_write> output_vertices: array<GeneratedVertex>;
@group(0) @binding(3) var<uniform> num_edges: u32;

// Build orthonormal basis perpendicular to cylinder axis
fn build_basis(axis: vec3<f32>) -> mat3x3<f32> {
    // Choose a vector not parallel to axis
    var up = vec3<f32>(0.0, 1.0, 0.0);
    if (abs(dot(axis, up)) > 0.99) {
        up = vec3<f32>(1.0, 0.0, 0.0);
    }

    let perp1 = normalize(cross(axis, up));
    let perp2 = cross(axis, perp1);

    return mat3x3<f32>(perp1, perp2, axis);
}

// Box vertex offsets (8 corners)
// Indices: 0-3 at tail, 4-7 at tip
fn get_box_corner(corner_id: u32, basis: mat3x3<f32>, tail: vec3<f32>, tip: vec3<f32>, radius: f32) -> vec3<f32> {
    let r = radius * 1.1;  // Slight padding to ensure coverage

    // Determine which end (tail or tip) and which corner
    let at_tip = corner_id >= 4u;
    let local_id = corner_id % 4u;

    // Corner offsets in local space
    var offset: vec2<f32>;
    switch (local_id) {
        case 0u: { offset = vec2<f32>(-r, -r); }
        case 1u: { offset = vec2<f32>( r, -r); }
        case 2u: { offset = vec2<f32>( r,  r); }
        case 3u: { offset = vec2<f32>(-r,  r); }
        default: { offset = vec2<f32>(0.0, 0.0); }
    }

    let base_pos = select(tail, tip, at_tip);
    // Extend slightly beyond endpoints to cover caps
    let axis_extend = select(-0.1 * radius, 0.1 * radius, at_tip);
    let axis_dir = normalize(tip - tail);

    return base_pos + basis[0] * offset.x + basis[1] * offset.y + axis_dir * axis_extend;
}

// Triangle indices for a box (12 triangles = 36 vertices)
// Returns corner index for given triangle vertex
fn get_box_triangle_vertex(tri_id: u32, vert_id: u32) -> u32 {
    // 12 triangles, each with 3 vertices
    // Front face (at tail): 0,1,2, 0,2,3
    // Back face (at tip): 4,6,5, 4,7,6
    // Top face: 3,2,6, 3,6,7
    // Bottom face: 0,5,1, 0,4,5
    // Right face: 1,5,6, 1,6,2
    // Left face: 0,3,7, 0,7,4

    var indices = array<u32, 36>(
        // Front (tail end, facing -Z in local)
        0u, 2u, 1u,  0u, 3u, 2u,
        // Back (tip end, facing +Z in local)
        4u, 5u, 6u,  4u, 6u, 7u,
        // Top
        3u, 6u, 2u,  3u, 7u, 6u,
        // Bottom
        0u, 1u, 5u,  0u, 5u, 4u,
        // Right
        1u, 2u, 6u,  1u, 6u, 5u,
        // Left
        0u, 4u, 7u,  0u, 7u, 3u
    );

    return indices[tri_id * 3u + vert_id];
}

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let edge_id = global_id.x;

    if (edge_id >= num_edges) {
        return;
    }

    // Read edge endpoints
    let tail = edge_vertices[edge_id * 2u].xyz;
    let tip = edge_vertices[edge_id * 2u + 1u].xyz;

    // Handle degenerate edges
    let edge_length = length(tip - tail);
    if (edge_length < 0.0001) {
        // Write degenerate triangles (all at same point)
        for (var i = 0u; i < 36u; i++) {
            let out_idx = edge_id * 36u + i;
            output_vertices[out_idx].position = vec4<f32>(tail, 1.0);
            output_vertices[out_idx].edge_id_and_vertex_id = vec4<u32>(edge_id, i, 0u, 0u);
        }
        return;
    }

    // Build orthonormal basis
    let axis = normalize(tip - tail);
    let basis = build_basis(axis);

    // Generate 36 vertices (12 triangles)
    for (var tri = 0u; tri < 12u; tri++) {
        for (var v = 0u; v < 3u; v++) {
            let corner_id = get_box_triangle_vertex(tri, v);
            let position = get_box_corner(corner_id, basis, tail, tip, uniforms.radius);

            let out_idx = edge_id * 36u + tri * 3u + v;
            output_vertices[out_idx].position = vec4<f32>(position, 1.0);
            output_vertices[out_idx].edge_id_and_vertex_id = vec4<u32>(edge_id, tri * 3u + v, 0u, 0u);
        }
    }
}
