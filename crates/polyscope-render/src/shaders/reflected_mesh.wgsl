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
    _pad1_0: f32,
    _pad1_1: f32,
    _pad1_2: f32,
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
@group(1) @binding(0) var<uniform> reflection: ReflectionUniforms;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) barycentric: vec3<f32>,
    @location(3) vertex_color: vec4<f32>,
    @location(4) edge_real: vec3<f32>,
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

    // Transform and flip normal for reflected geometry
    let world_normal = normalize((mesh_uniforms.model * vec4<f32>(local_normal, 0.0)).xyz);
    let reflected_normal = -world_normal; // Flip for reflection

    out.clip_position = camera.view_proj * vec4<f32>(reflected_position, 1.0);
    out.world_position = reflected_position;
    out.world_normal = reflected_normal;
    out.barycentric = bary;
    out.vertex_color = color;
    out.edge_real = edge_is_real[vertex_index].xyz;

    return out;
}

@fragment
fn fs_main(in: VertexOutput, @builtin(front_facing) front_facing: bool) -> @location(0) vec4<f32> {
    // Clip pixels below ground plane (reflected geometry should not poke through)
    if (in.world_position.y < reflection.ground_height) {
        discard;
    }

    // Get base color
    var base_color = in.vertex_color.rgb;
    if (in.vertex_color.a < 0.001) {
        base_color = mesh_uniforms.surface_color.rgb;
    }

    // Simple lighting
    let light_dir = normalize(vec3<f32>(0.5, 1.0, 0.3));
    let view_dir = normalize(camera.camera_pos - in.world_position);

    // Use flipped normal for front-facing test on reflected geometry
    // Since we flipped normals, front_facing logic is inverted
    let normal = select(-in.world_normal, in.world_normal, front_facing);

    // Ambient + diffuse
    let ambient = 0.3;
    let diffuse = max(dot(normal, light_dir), 0.0) * 0.6;

    // Specular
    let half_vec = normalize(light_dir + view_dir);
    let specular = pow(max(dot(normal, half_vec), 0.0), 32.0) * 0.2;

    var final_color = base_color * (ambient + diffuse) + vec3<f32>(specular);

    // Apply reflection intensity (fade out reflection)
    final_color *= reflection.intensity;

    return vec4<f32>(final_color, reflection.intensity);
}
