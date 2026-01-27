// Surface mesh OIT (Order-Independent Transparency) accumulation shader
// Outputs weighted color to accumulation buffer and transmittance to reveal buffer.

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

struct MeshUniforms {
    model: mat4x4<f32>,
    shade_style: u32,      // 0 = smooth, 1 = flat, 2 = tri-flat
    show_edges: u32,       // 0 = off, 1 = on
    edge_width: f32,
    transparency: f32,
    surface_color: vec4<f32>,
    edge_color: vec4<f32>,
    backface_policy: u32,  // 0 = identical, 1 = different, 2 = custom, 3 = cull
    slice_planes_enabled: u32, // 0 = off, 1 = on
    _pad1_0: f32,
    _pad1_1: f32,
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
@group(0) @binding(6) var<storage, read> edge_is_real: array<vec4<f32>>;

@group(1) @binding(0) var<uniform> slice_planes: SlicePlanesArray;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) barycentric: vec3<f32>,
    @location(3) vertex_color: vec4<f32>,
    @location(4) edge_real: vec3<f32>,
}

// OIT output: accumulation texture and reveal texture
struct OitOutput {
    @location(0) accum: vec4<f32>,
    @location(1) reveal: vec4<f32>,  // Only .r is used, but vec4 is safer for some drivers
}

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_index: u32,
) -> VertexOutput {
    var out: VertexOutput;

    // Read position, normal, barycentric, color from storage buffers
    let local_position = positions[vertex_index].xyz;
    let local_normal = normals[vertex_index].xyz;
    let bary = barycentrics[vertex_index].xyz;
    let color = colors[vertex_index];

    // Apply model transform
    let world_position = (mesh_uniforms.model * vec4<f32>(local_position, 1.0)).xyz;
    // Transform normal with upper-left 3x3 of model matrix (assuming uniform scale)
    let world_normal = normalize((mesh_uniforms.model * vec4<f32>(local_normal, 0.0)).xyz);

    // Transform position with view_proj matrix
    out.clip_position = camera.view_proj * vec4<f32>(world_position, 1.0);
    out.world_position = world_position;
    out.world_normal = world_normal;
    out.barycentric = bary;
    out.vertex_color = color;
    out.edge_real = edge_is_real[vertex_index].xyz;

    return out;
}

// Weight function for OIT (simplified version)
// The original McGuire/Bavoil weight function was too aggressive.
// This simpler version provides depth-based ordering without extreme values.
fn oit_weight(depth: f32, alpha: f32) -> f32 {
    // Simple depth-based weight: closer objects get higher weight
    // depth is in [0, 1] where 0 is near, 1 is far
    let z = 1.0 - depth; // invert so near = 1, far = 0
    return max(0.01, z * z * z * 1000.0);
}

@fragment
fn fs_main(
    in: VertexOutput,
    @builtin(front_facing) front_facing: bool,
) -> OitOutput {
    // Handle backface culling: if policy==3 and !front_facing, discard
    if (mesh_uniforms.backface_policy == 3u && !front_facing) {
        discard;
    }

    // Slice plane culling - discard fragments on negative side of enabled planes
    if (mesh_uniforms.slice_planes_enabled != 0u) {
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

    // Determine base color based on backface policy
    var base_color = mesh_uniforms.surface_color.rgb;
    if (!front_facing) {
        switch (mesh_uniforms.backface_policy) {
            case 0u: {
                base_color = mesh_uniforms.surface_color.rgb;
            }
            case 1u: {
                base_color = mesh_uniforms.surface_color.rgb * 0.5;
            }
            case 2u: {
                base_color = mesh_uniforms.backface_color.rgb;
            }
            default: {
                base_color = mesh_uniforms.surface_color.rgb;
            }
        }
    }

    // Use per-vertex color if non-zero (for quantities)
    let color_sum = in.vertex_color.r + in.vertex_color.g + in.vertex_color.b;
    if (color_sum > 0.001) {
        base_color = in.vertex_color.rgb;
    }

    // Calculate normal for lighting based on shade_style
    var normal: vec3<f32>;

    if (mesh_uniforms.shade_style == 0u) {
        normal = normalize(in.world_normal);
    } else {
        let dpdx_pos = dpdx(in.world_position);
        let dpdy_pos = dpdy(in.world_position);
        normal = normalize(cross(dpdx_pos, dpdy_pos));

        let view_dir = normalize(camera.camera_pos - in.world_position);
        if (dot(normal, view_dir) < 0.0) {
            normal = -normal;
        }
    }

    if (!front_facing) {
        normal = -normal;
    }

    // Apply lighting
    let light_dir = normalize(vec3<f32>(0.3, 0.5, 1.0));
    let ambient = 0.5;
    let diffuse = 0.5 * max(dot(normal, light_dir), 0.0);
    let lighting = ambient + diffuse;

    var color = base_color * lighting;

    // Wireframe
    if (mesh_uniforms.show_edges == 1u) {
        let bary = in.barycentric;
        let edge_real = in.edge_real;

        var d = 1.0;
        if (edge_real.z > 0.5) {
            d = min(d, bary.z);
        }
        if (edge_real.x > 0.5) {
            d = min(d, bary.x);
        }
        if (edge_real.y > 0.5) {
            d = min(d, bary.y);
        }

        let edge_factor = smoothstep(0.0, mesh_uniforms.edge_width * fwidth(d), d);
        color = mix(mesh_uniforms.edge_color.rgb, color, edge_factor);
    }

    // Compute alpha from transparency
    // transparency of 0.0 means fully opaque (alpha = 1.0)
    // transparency of 1.0 means fully transparent (alpha = 0.0)
    let alpha = 1.0 - mesh_uniforms.transparency;

    // Skip nearly transparent fragments
    if (alpha < 0.001) {
        discard;
    }

    // Compute OIT weight based on depth
    let depth = in.clip_position.z; // Normalized depth [0, 1]
    let weight = oit_weight(depth, alpha);

    // OIT output
    var out: OitOutput;
    // Accumulation: premultiplied color * weight
    out.accum = vec4<f32>(color * alpha, alpha) * weight;
    // Reveal: alpha (will be multiplied via blend state to compute product)
    // Output as vec4 for compatibility - only .r channel is used
    out.reveal = vec4<f32>(alpha, 0.0, 0.0, 1.0);

    return out;
}
