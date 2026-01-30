// Reflected mesh shader
// Renders mesh geometry with reflection matrix applied
// Flips normals for correct lighting on reflected geometry

struct CameraUniforms {
    view: mat4x4<f32>,
    proj: mat4x4<f32>,
    view_proj: mat4x4<f32>,
    inv_proj: mat4x4<f32>,
    camera_pos: vec3<f32>,
    _padding: f32,
}

struct MeshUniforms {
    model: mat4x4<f32>,
    shade_style: u32,
    show_edges: u32,
    edge_width: f32,
    transparency: f32,
    surface_color: vec4<f32>,
    edge_color: vec4<f32>,
    backface_policy: u32,
    slice_planes_enabled: u32,
    use_vertex_color: u32,
    _pad1: f32,
    _pad2_0: f32,
    _pad2_1: f32,
    _pad2_2: f32,
    _pad3: f32,
    backface_color: vec4<f32>,
}

struct ReflectionUniforms {
    reflection_matrix: mat4x4<f32>,
    intensity: f32,
    ground_height: f32,
    _padding: vec2<f32>,
}

@group(0) @binding(0) var<uniform> camera: CameraUniforms;
@group(0) @binding(1) var<uniform> mesh_uniforms: MeshUniforms;
@group(0) @binding(2) var<storage, read> positions: array<vec4<f32>>;
@group(0) @binding(3) var<storage, read> normals: array<vec4<f32>>;
@group(0) @binding(4) var<storage, read> barycentrics: array<vec4<f32>>;
@group(0) @binding(5) var<storage, read> colors: array<vec4<f32>>;
@group(0) @binding(6) var<storage, read> edge_is_real: array<vec4<f32>>;
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

@group(1) @binding(0) var<uniform> reflection: ReflectionUniforms;

// Matcap textures (Group 2)
@group(2) @binding(0) var matcap_r: texture_2d<f32>;
@group(2) @binding(1) var matcap_g: texture_2d<f32>;
@group(2) @binding(2) var matcap_b: texture_2d<f32>;
@group(2) @binding(3) var matcap_k: texture_2d<f32>;
@group(2) @binding(4) var matcap_sampler: sampler;

// Slice planes (Group 3)
@group(3) @binding(0) var<uniform> slice_planes: SlicePlanesArray;

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
    @location(2) barycentric: vec3<f32>,
    @location(3) vertex_color: vec4<f32>,
    @location(4) edge_real: vec3<f32>,
    @location(5) original_world_position: vec3<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var out: VertexOutput;

    // Read position and normal from storage buffers
    let local_position = positions[vertex_index].xyz;
    let local_normal = normals[vertex_index].xyz;
    let bary = barycentrics[vertex_index].xyz;
    let color = colors[vertex_index];

    // Apply model transform
    let world_position = (mesh_uniforms.model * vec4<f32>(local_position, 1.0)).xyz;

    // Apply reflection matrix
    let reflected_position = (reflection.reflection_matrix * vec4<f32>(world_position, 1.0)).xyz;

    // Transform normal through model, then reflect through reflection plane
    let world_normal = normalize((mesh_uniforms.model * vec4<f32>(local_normal, 0.0)).xyz);
    let reflected_normal = normalize((reflection.reflection_matrix * vec4<f32>(world_normal, 0.0)).xyz);

    out.clip_position = camera.view_proj * vec4<f32>(reflected_position, 1.0);
    out.world_position = reflected_position;
    out.world_normal = reflected_normal;
    out.barycentric = bary;
    out.vertex_color = color;
    out.edge_real = edge_is_real[vertex_index].xyz;
    out.original_world_position = world_position;

    return out;
}

@fragment
fn fs_main(in: VertexOutput, @builtin(front_facing) front_facing: bool) -> @location(0) vec4<f32> {
    // Clip pixels above ground plane (reflected geometry should not poke through)
    if (in.world_position.y > reflection.ground_height) {
        discard;
    }

    // Slice plane culling — test against original (pre-reflection) world position
    if (mesh_uniforms.slice_planes_enabled != 0u) {
        for (var i = 0u; i < 4u; i = i + 1u) {
            let plane = slice_planes.planes[i];
            if (plane.enabled > 0.5) {
                let dist = dot(in.original_world_position - plane.origin, plane.normal);
                if (dist < 0.0) {
                    discard;
                }
            }
        }
    }

    // Get base color
    var base_color = mesh_uniforms.surface_color.rgb;
    if (mesh_uniforms.use_vertex_color == 1u) {
        base_color = in.vertex_color.rgb;
    }

    // Use flipped normal for front-facing test on reflected geometry
    // Since we flipped normals, front_facing logic is inverted
    let normal = select(-in.world_normal, in.world_normal, front_facing);

    // Matcap lighting: transform normal to view space
    let view_normal = normalize((camera.view * vec4<f32>(normal, 0.0)).xyz);
    let final_color = light_surface_matcap(view_normal, base_color);

    // Output with reflection intensity as alpha
    // Standard alpha blending will do: reflection * intensity + ground * (1 - intensity)
    return vec4<f32>(final_color, reflection.intensity);
}
