// Vector arrow shader using instanced rendering
// Each instance is an arrow from a base point in a direction

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

// Arrow cylinder mesh (simplified - 8 segments)
// This creates a cylinder from (0,0,0) to (0,0,1) with radius 1
const CYLINDER_SEGMENTS: u32 = 8u;

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_index: u32,
    @builtin(instance_index) instance_index: u32,
) -> VertexOutput {
    var out: VertexOutput;

    let base_pos = base_positions[instance_index];
    let vec = vectors[instance_index];
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
    let radius = vector_uniforms.radius;

    // Build orthonormal basis
    var up = vec3<f32>(0.0, 1.0, 0.0);
    if (abs(dot(vec_dir, up)) > 0.99) {
        up = vec3<f32>(1.0, 0.0, 0.0);
    }
    let right = normalize(cross(vec_dir, up));
    let forward = cross(right, vec_dir);

    // Generate cylinder vertex
    let segment = vertex_index / 6u;
    let tri_vert = vertex_index % 6u;

    let angle0 = f32(segment) / f32(CYLINDER_SEGMENTS) * 6.283185;
    let angle1 = f32(segment + 1u) / f32(CYLINDER_SEGMENTS) * 6.283185;

    var local_pos: vec3<f32>;
    var local_normal: vec3<f32>;

    // Two triangles per segment
    if (tri_vert == 0u) {
        local_pos = vec3<f32>(cos(angle0) * radius, sin(angle0) * radius, 0.0);
        local_normal = vec3<f32>(cos(angle0), sin(angle0), 0.0);
    } else if (tri_vert == 1u) {
        local_pos = vec3<f32>(cos(angle1) * radius, sin(angle1) * radius, 0.0);
        local_normal = vec3<f32>(cos(angle1), sin(angle1), 0.0);
    } else if (tri_vert == 2u) {
        local_pos = vec3<f32>(cos(angle0) * radius, sin(angle0) * radius, scaled_length);
        local_normal = vec3<f32>(cos(angle0), sin(angle0), 0.0);
    } else if (tri_vert == 3u) {
        local_pos = vec3<f32>(cos(angle1) * radius, sin(angle1) * radius, 0.0);
        local_normal = vec3<f32>(cos(angle1), sin(angle1), 0.0);
    } else if (tri_vert == 4u) {
        local_pos = vec3<f32>(cos(angle1) * radius, sin(angle1) * radius, scaled_length);
        local_normal = vec3<f32>(cos(angle1), sin(angle1), 0.0);
    } else {
        local_pos = vec3<f32>(cos(angle0) * radius, sin(angle0) * radius, scaled_length);
        local_normal = vec3<f32>(cos(angle0), sin(angle0), 0.0);
    }

    // Transform to world space
    let world_pos = base_pos
        + right * local_pos.x
        + forward * local_pos.y
        + vec_dir * local_pos.z;

    let world_normal = right * local_normal.x + forward * local_normal.y;

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
