// Vector arrow shader using instanced rendering
// Each instance is an arrow (cylinder shaft + cone head) from a base point in a direction

struct CameraUniforms {
    view: mat4x4<f32>,
    proj: mat4x4<f32>,
    view_proj: mat4x4<f32>,
    inv_proj: mat4x4<f32>,
    camera_pos: vec3<f32>,
    _padding: f32,
}

// Slice plane uniforms for fragment-level slicing
struct SlicePlaneUniforms {
    origin: vec3<f32>,
    enabled: f32,
    normal: vec3<f32>,
    _padding: f32,
}

struct SlicePlanesArray {
    planes: array<SlicePlaneUniforms, 4>,
}

struct VectorUniforms {
    model: mat4x4<f32>,
    length_scale: f32,
    radius: f32,
    _padding: vec2<f32>,
    color: vec4<f32>,
}

@group(0) @binding(0) var<uniform> camera: CameraUniforms;
@group(0) @binding(1) var<uniform> vector_uniforms: VectorUniforms;
@group(0) @binding(2) var<storage, read> base_positions: array<vec3<f32>>;
@group(0) @binding(3) var<storage, read> vectors: array<vec3<f32>>;

@group(1) @binding(0) var<uniform> slice_planes: SlicePlanesArray;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) color: vec3<f32>,
    @location(2) world_position: vec3<f32>,
}

// Arrow geometry: cylinder shaft + cone arrowhead
// Shaft: 8 segments × 6 verts = 48 verts (indices 0..47)
// Cone:  8 segments × 3 verts = 24 verts (indices 48..71)
// Total: 72 vertices per arrow instance
const SEGMENTS: u32 = 8u;
const SHAFT_VERTS: u32 = 48u; // SEGMENTS * 6

// Arrow proportions
const CONE_HEIGHT_FRAC: f32 = 0.3;     // Cone takes 30% of total arrow length
const CONE_RADIUS_MULT: f32 = 2.0;     // Cone base radius = 2× shaft radius

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_index: u32,
    @builtin(instance_index) instance_index: u32,
) -> VertexOutput {
    var out: VertexOutput;

    // Transform base position and vector direction by model matrix
    let base_pos = (vector_uniforms.model * vec4<f32>(base_positions[instance_index], 1.0)).xyz;
    let raw_vec = vectors[instance_index];
    let vec = (vector_uniforms.model * vec4<f32>(raw_vec, 0.0)).xyz;
    let vec_length = length(vec);

    if (vec_length < 0.0001) {
        // Zero vector - place vertex at origin (will be degenerate)
        out.clip_position = camera.view_proj * vec4<f32>(base_pos, 1.0);
        out.normal = vec3<f32>(0.0, 1.0, 0.0);
        out.color = vector_uniforms.color.rgb;
        out.world_position = base_pos;
        return out;
    }

    let vec_dir = vec / vec_length;
    let scaled_length = vec_length * vector_uniforms.length_scale;
    let shaft_radius = vector_uniforms.radius;
    let cone_base_radius = shaft_radius * CONE_RADIUS_MULT;
    let shaft_height = scaled_length * (1.0 - CONE_HEIGHT_FRAC);
    let cone_height = scaled_length * CONE_HEIGHT_FRAC;

    // Build right-handed orthonormal basis (right × forward = vec_dir)
    var up = vec3<f32>(0.0, 1.0, 0.0);
    if (abs(dot(vec_dir, up)) > 0.99) {
        up = vec3<f32>(1.0, 0.0, 0.0);
    }
    let right = normalize(cross(up, vec_dir));
    let forward = cross(vec_dir, right);

    var local_pos: vec3<f32>;
    var local_normal: vec3<f32>;

    if (vertex_index < SHAFT_VERTS) {
        // === Cylinder shaft ===
        let segment = vertex_index / 6u;
        let tri_vert = vertex_index % 6u;

        let angle0 = f32(segment) / f32(SEGMENTS) * 6.283185;
        let angle1 = f32(segment + 1u) / f32(SEGMENTS) * 6.283185;

        // Two triangles per segment forming a quad
        if (tri_vert == 0u) {
            local_pos = vec3<f32>(cos(angle0) * shaft_radius, sin(angle0) * shaft_radius, 0.0);
            local_normal = vec3<f32>(cos(angle0), sin(angle0), 0.0);
        } else if (tri_vert == 1u) {
            local_pos = vec3<f32>(cos(angle1) * shaft_radius, sin(angle1) * shaft_radius, 0.0);
            local_normal = vec3<f32>(cos(angle1), sin(angle1), 0.0);
        } else if (tri_vert == 2u) {
            local_pos = vec3<f32>(cos(angle0) * shaft_radius, sin(angle0) * shaft_radius, shaft_height);
            local_normal = vec3<f32>(cos(angle0), sin(angle0), 0.0);
        } else if (tri_vert == 3u) {
            local_pos = vec3<f32>(cos(angle1) * shaft_radius, sin(angle1) * shaft_radius, 0.0);
            local_normal = vec3<f32>(cos(angle1), sin(angle1), 0.0);
        } else if (tri_vert == 4u) {
            local_pos = vec3<f32>(cos(angle1) * shaft_radius, sin(angle1) * shaft_radius, shaft_height);
            local_normal = vec3<f32>(cos(angle1), sin(angle1), 0.0);
        } else {
            local_pos = vec3<f32>(cos(angle0) * shaft_radius, sin(angle0) * shaft_radius, shaft_height);
            local_normal = vec3<f32>(cos(angle0), sin(angle0), 0.0);
        }
    } else {
        // === Cone arrowhead ===
        let cone_index = vertex_index - SHAFT_VERTS;
        let segment = cone_index / 3u;
        let tri_vert = cone_index % 3u;

        let angle0 = f32(segment) / f32(SEGMENTS) * 6.283185;
        let angle1 = f32(segment + 1u) / f32(SEGMENTS) * 6.283185;

        // Cone normal: for a cone with base radius R and height h,
        // the outward normal has radial component h and axial component R,
        // normalized to unit length.
        let cone_norm_len = sqrt(cone_height * cone_height + cone_base_radius * cone_base_radius);
        let n_radial = cone_height / cone_norm_len;
        let n_axial = cone_base_radius / cone_norm_len;

        // One triangle per segment: two base verts + tip
        if (tri_vert == 0u) {
            // Base vertex at angle0
            local_pos = vec3<f32>(cos(angle0) * cone_base_radius, sin(angle0) * cone_base_radius, shaft_height);
            local_normal = vec3<f32>(cos(angle0) * n_radial, sin(angle0) * n_radial, n_axial);
        } else if (tri_vert == 1u) {
            // Base vertex at angle1
            local_pos = vec3<f32>(cos(angle1) * cone_base_radius, sin(angle1) * cone_base_radius, shaft_height);
            local_normal = vec3<f32>(cos(angle1) * n_radial, sin(angle1) * n_radial, n_axial);
        } else {
            // Tip vertex
            local_pos = vec3<f32>(0.0, 0.0, scaled_length);
            // Average normal for tip (use midpoint angle)
            let mid_angle = (angle0 + angle1) * 0.5;
            local_normal = vec3<f32>(cos(mid_angle) * n_radial, sin(mid_angle) * n_radial, n_axial);
        }
    }

    // Transform to world space
    let world_pos = base_pos
        + right * local_pos.x
        + forward * local_pos.y
        + vec_dir * local_pos.z;

    let world_normal = normalize(right * local_normal.x + forward * local_normal.y + vec_dir * local_normal.z);

    out.clip_position = camera.view_proj * vec4<f32>(world_pos, 1.0);
    out.normal = world_normal;
    out.color = vector_uniforms.color.rgb;
    out.world_position = world_pos;

    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Slice plane culling
    for (var i = 0u; i < 4u; i = i + 1u) {
        let plane = slice_planes.planes[i];
        if (plane.enabled > 0.5) {
            let dist = dot(in.world_position - plane.origin, plane.normal);
            if (dist < 0.0) {
                discard;
            }
        }
    }

    let light_dir = normalize(vec3<f32>(0.3, 0.5, 1.0));
    let ambient = 0.3;
    let diffuse = max(dot(normalize(in.normal), light_dir), 0.0) * 0.7;
    let lighting = ambient + diffuse;

    return vec4<f32>(in.color * lighting, 1.0);
}
