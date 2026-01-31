// Gridcube shader for volume grid scalar visualization
// Renders instanced unit cubes, one per grid node/cell
// Each cube is colored by mapping its scalar value through a colormap

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

struct GridcubeUniforms {
    model: mat4x4<f32>,
    cube_size_factor: f32,  // 0..1, shrink factor for cubes
    data_min: f32,          // scalar range min
    data_max: f32,          // scalar range max
    transparency: f32,
    slice_planes_enabled: u32,
    _pad0: f32,
    _pad1: f32,
    _pad2: f32,
}

// Group 0: per-object data
@group(0) @binding(0) var<uniform> camera: CameraUniforms;
@group(0) @binding(1) var<uniform> uniforms: GridcubeUniforms;
@group(0) @binding(2) var<storage, read> cube_positions: array<vec4<f32>>;  // xyz = center, w = unused
@group(0) @binding(3) var<storage, read> cube_normals: array<vec4<f32>>;    // face normals for the unit cube template
@group(0) @binding(4) var<storage, read> scalar_values: array<f32>;         // one per instance
@group(0) @binding(5) var colormap_texture: texture_1d<f32>;
@group(0) @binding(6) var colormap_sampler: sampler;

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
    @location(2) scalar_t: f32,  // normalized scalar for colormap lookup
}

struct FragmentOutput {
    @location(0) color: vec4<f32>,
    @location(1) normal: vec4<f32>,
}

// Unit cube has 36 vertices (12 triangles, 6 faces)
// vertex_index % 36 = local vertex within cube template
// vertex_index / 36 = instance index
const VERTS_PER_CUBE: u32 = 36u;

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_index: u32,
) -> VertexOutput {
    var out: VertexOutput;

    let instance_id = vertex_index / VERTS_PER_CUBE;
    let local_vertex_id = vertex_index % VERTS_PER_CUBE;

    // Read cube template position and normal from storage buffers
    let local_pos = cube_positions[local_vertex_id].xyz;
    let local_normal = cube_normals[local_vertex_id].xyz;

    // Read instance data: center position and spacing from the instance buffer
    // Instance positions are packed as vec4: xyz = center, w = spacing (half-size)
    let instance_data = cube_positions[VERTS_PER_CUBE + instance_id];
    let center = instance_data.xyz;
    let half_size = instance_data.w * uniforms.cube_size_factor;

    // Scale and translate the unit cube to instance position
    let scaled_pos = local_pos * half_size + center;

    // Apply model transform
    let world_position = (uniforms.model * vec4<f32>(scaled_pos, 1.0)).xyz;
    let world_normal = normalize((uniforms.model * vec4<f32>(local_normal, 0.0)).xyz);

    out.clip_position = camera.view_proj * vec4<f32>(world_position, 1.0);
    out.world_position = world_position;
    out.world_normal = world_normal;

    // Normalize scalar to [0,1] for colormap lookup
    let scalar = scalar_values[instance_id];
    let range = uniforms.data_max - uniforms.data_min;
    if (range > 0.0) {
        out.scalar_t = clamp((scalar - uniforms.data_min) / range, 0.0, 1.0);
    } else {
        out.scalar_t = 0.5;
    }

    return out;
}

@fragment
fn fs_main(
    in: VertexOutput,
    @builtin(front_facing) front_facing: bool,
) -> FragmentOutput {
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

    // Use the flat face normal from the cube (already correct per-face)
    var normal = normalize(in.world_normal);
    if (!front_facing) {
        normal = -normal;
    }

    // Sample colormap to get base color
    let base_color = textureSample(colormap_texture, colormap_sampler, in.scalar_t).rgb;

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
