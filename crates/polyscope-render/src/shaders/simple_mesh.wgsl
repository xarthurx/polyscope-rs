// Simple mesh shader for isosurface rendering
// Minimal: positions + normals + uniform base color + matcap lighting
// No per-vertex colors, no wireframe, no barycentrics

struct CameraUniforms {
    view: mat4x4<f32>,
    proj: mat4x4<f32>,
    view_proj: mat4x4<f32>,
    inv_proj: mat4x4<f32>,
    camera_pos: vec3<f32>,
    _padding: f32,
}

struct SlicePlaneUniforms {
    origin: vec3<f32>,
    enabled: f32,
    normal: vec3<f32>,
    _padding: f32,
}

struct SlicePlanesArray {
    planes: array<SlicePlaneUniforms, 4>,
}

struct SimpleMeshUniforms {
    model: mat4x4<f32>,
    base_color: vec4<f32>,
    transparency: f32,
    slice_planes_enabled: u32,
    backface_policy: u32, // 0 = identical, 1 = different, 3 = cull
    _pad: f32,
}

// Group 0: per-object data
@group(0) @binding(0) var<uniform> camera: CameraUniforms;
@group(0) @binding(1) var<uniform> uniforms: SimpleMeshUniforms;
@group(0) @binding(2) var<storage, read> positions: array<vec4<f32>>;
@group(0) @binding(3) var<storage, read> normals: array<vec4<f32>>;

// Group 1: slice planes
@group(1) @binding(0) var<uniform> slice_planes: SlicePlanesArray;

// Group 2: matcap textures
@group(2) @binding(0) var matcap_r: texture_2d<f32>;
@group(2) @binding(1) var matcap_g: texture_2d<f32>;
@group(2) @binding(2) var matcap_b: texture_2d<f32>;
@group(2) @binding(3) var matcap_k: texture_2d<f32>;
@group(2) @binding(4) var matcap_sampler: sampler;

fn light_surface_matcap(normal: vec3<f32>, color: vec3<f32>) -> vec3<f32> {
    var n = normalize(normal);
    n.y = -n.y;
    n = n * 0.98;
    let uv = n.xy * 0.5 + vec2<f32>(0.5);
    let mat_r = textureSample(matcap_r, matcap_sampler, uv).rgb;
    let mat_g = textureSample(matcap_g, matcap_sampler, uv).rgb;
    let mat_b = textureSample(matcap_b, matcap_sampler, uv).rgb;
    let mat_k = textureSample(matcap_k, matcap_sampler, uv).rgb;
    return color.r * mat_r + color.g * mat_g
         + color.b * mat_b + (1.0 - color.r - color.g - color.b) * mat_k;
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) world_normal: vec3<f32>,
}

struct FragmentOutput {
    @location(0) color: vec4<f32>,
    @location(1) normal: vec4<f32>,
}

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_index: u32,
) -> VertexOutput {
    var out: VertexOutput;

    let local_position = positions[vertex_index].xyz;
    let local_normal = normals[vertex_index].xyz;

    let world_position = (uniforms.model * vec4<f32>(local_position, 1.0)).xyz;
    let world_normal = normalize((uniforms.model * vec4<f32>(local_normal, 0.0)).xyz);

    out.clip_position = camera.view_proj * vec4<f32>(world_position, 1.0);
    out.world_position = world_position;
    out.world_normal = world_normal;

    return out;
}

@fragment
fn fs_main(
    in: VertexOutput,
    @builtin(front_facing) front_facing: bool,
) -> FragmentOutput {
    // Backface culling
    if (uniforms.backface_policy == 3u && !front_facing) {
        discard;
    }

    // Slice plane culling
    if (uniforms.slice_planes_enabled != 0u) {
        for (var i = 0u; i < 4u; i = i + 1u) {
            let plane = slice_planes.planes[i];
            if (plane.enabled > 0.5) {
                let dist = dot(in.world_position - plane.origin, plane.normal);
                if (dist < 0.0) {
                    discard;
                }
            }
        }
    }

    // Use flat shading (screen-space derivatives) for isosurface
    let dpdx_pos = dpdx(in.world_position);
    let dpdy_pos = dpdy(in.world_position);
    var normal = normalize(cross(dpdx_pos, dpdy_pos));

    // Ensure normal faces camera
    let view_dir = normalize(camera.camera_pos - in.world_position);
    if (dot(normal, view_dir) < 0.0) {
        normal = -normal;
    }

    if (!front_facing) {
        normal = -normal;
    }

    // Determine base color
    var base_color = uniforms.base_color.rgb;
    if (!front_facing && uniforms.backface_policy == 1u) {
        base_color = base_color * 0.5;
    }

    // Matcap lighting
    let view_normal_for_matcap = normalize((camera.view * vec4<f32>(normal, 0.0)).xyz);
    let color = light_surface_matcap(view_normal_for_matcap, base_color);

    let alpha = 1.0 - uniforms.transparency;
    if (alpha <= 0.0) {
        discard;
    }

    // View-space normal for SSAO
    let view_normal = (camera.view * vec4<f32>(normal, 0.0)).xyz;

    var out: FragmentOutput;
    out.color = vec4<f32>(color, alpha);
    out.normal = vec4<f32>(view_normal * 0.5 + 0.5, alpha);
    return out;
}
