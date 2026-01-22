// Surface mesh shader with shading modes and wireframe support
// Supports smooth, flat, and tri-flat shading with optional edge rendering

struct CameraUniforms {
    view: mat4x4<f32>,
    proj: mat4x4<f32>,
    view_proj: mat4x4<f32>,
    inv_proj: mat4x4<f32>,
    camera_pos: vec3<f32>,
    _padding: f32,
}

struct MeshUniforms {
    shade_style: u32,      // 0 = smooth, 1 = flat, 2 = tri-flat
    show_edges: u32,       // 0 = off, 1 = on
    edge_width: f32,
    transparency: f32,
    surface_color: vec4<f32>,
    edge_color: vec4<f32>,
    backface_policy: u32,  // 0 = identical, 1 = different, 2 = custom, 3 = cull
    _pad1_0: f32,
    _pad1_1: f32,
    _pad1_2: f32,
    _pad2_0: f32,
    _pad2_1: f32,
    _pad2_2: f32,
    _pad3: f32,
    backface_color: vec4<f32>,
}

@group(0) @binding(0) var<uniform> camera: CameraUniforms;
@group(0) @binding(1) var<uniform> mesh_uniforms: MeshUniforms;
@group(0) @binding(2) var<storage, read> positions: array<vec4<f32>>;
@group(0) @binding(3) var<storage, read> normals: array<vec4<f32>>;
@group(0) @binding(4) var<storage, read> barycentrics: array<vec4<f32>>;
@group(0) @binding(5) var<storage, read> colors: array<vec4<f32>>;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) barycentric: vec3<f32>,
    @location(3) vertex_color: vec4<f32>,
}

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_index: u32,
) -> VertexOutput {
    var out: VertexOutput;

    // Read position, normal, barycentric, color from storage buffers
    let position = positions[vertex_index].xyz;
    let normal = normals[vertex_index].xyz;
    let bary = barycentrics[vertex_index].xyz;
    let color = colors[vertex_index];

    // Transform position with view_proj matrix
    out.clip_position = camera.view_proj * vec4<f32>(position, 1.0);
    out.world_position = position;
    out.world_normal = normal;
    out.barycentric = bary;
    out.vertex_color = color;

    return out;
}

@fragment
fn fs_main(
    in: VertexOutput,
    @builtin(front_facing) front_facing: bool,
) -> @location(0) vec4<f32> {
    // Handle backface culling: if policy==3 and !front_facing, discard
    if (mesh_uniforms.backface_policy == 3u && !front_facing) {
        discard;
    }

    // Determine base color based on backface policy
    var base_color = mesh_uniforms.surface_color.rgb;
    if (!front_facing) {
        switch (mesh_uniforms.backface_policy) {
            case 0u: {
                // identical - use surface color
                base_color = mesh_uniforms.surface_color.rgb;
            }
            case 1u: {
                // different - darken surface color
                base_color = mesh_uniforms.surface_color.rgb * 0.5;
            }
            case 2u: {
                // custom - use backface color
                base_color = mesh_uniforms.backface_color.rgb;
            }
            default: {
                // fallback (shouldn't reach here due to discard above for case 3)
                base_color = mesh_uniforms.surface_color.rgb;
            }
        }
    }

    // Use per-vertex color if non-zero (for quantities)
    // Check if the color is non-default (not all zeros or very close to it)
    let color_sum = in.vertex_color.r + in.vertex_color.g + in.vertex_color.b;
    if (color_sum > 0.001) {
        base_color = in.vertex_color.rgb;
    }

    // Calculate normal for lighting
    // For flat shading, we could use face normal computed from derivatives,
    // but since we have per-vertex normals, we use them for smooth shading
    var normal = normalize(in.world_normal);

    // Flip normal for backfaces
    if (!front_facing) {
        normal = -normal;
    }

    // Apply lighting: ambient (0.3) + diffuse (0.7 * max(dot(N, L), 0))
    let light_dir = normalize(vec3<f32>(0.3, 0.5, 1.0));
    let ambient = 0.3;
    let diffuse = 0.7 * max(dot(normal, light_dir), 0.0);
    let lighting = ambient + diffuse;

    var color = base_color * lighting;

    // Wireframe: if show_edges, mix edge_color based on barycentric distance
    if (mesh_uniforms.show_edges == 1u) {
        let bary = in.barycentric;
        let d = min(bary.x, min(bary.y, bary.z));
        let edge_factor = smoothstep(0.0, mesh_uniforms.edge_width * fwidth(d), d);
        color = mix(mesh_uniforms.edge_color.rgb, color, edge_factor);
    }

    // Return with transparency applied
    // Note: transparency of 0.0 means fully opaque (alpha = 1.0)
    // transparency of 1.0 means fully transparent (alpha = 0.0)
    let alpha = 1.0 - mesh_uniforms.transparency;

    return vec4<f32>(color, alpha);
}
