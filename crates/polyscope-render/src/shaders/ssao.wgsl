// SSAO (Screen Space Ambient Occlusion) shader
// Classic Crytek-style SSAO with improvements for edge handling

struct SsaoUniforms {
    proj: mat4x4<f32>,
    inv_proj: mat4x4<f32>,
    radius: f32,
    bias: f32,
    intensity: f32,
    sample_count: u32,
    screen_width: f32,
    screen_height: f32,
    _padding: vec2<f32>,
}

// 32 well-distributed hemisphere samples (sufficient for good quality)
// Samples are in tangent space with Z pointing up (along normal)
const KERNEL_SIZE: u32 = 32u;
var<private> kernel: array<vec3<f32>, 32> = array<vec3<f32>, 32>(
    // Inner samples (close to center, high contribution)
    vec3<f32>( 0.05,  0.02,  0.05),
    vec3<f32>(-0.04,  0.05,  0.06),
    vec3<f32>( 0.03, -0.06,  0.04),
    vec3<f32>(-0.02, -0.04,  0.07),
    vec3<f32>( 0.08,  0.04,  0.08),
    vec3<f32>(-0.07,  0.06,  0.05),
    vec3<f32>( 0.05, -0.08,  0.06),
    vec3<f32>(-0.06, -0.05,  0.09),
    // Middle samples
    vec3<f32>( 0.12,  0.03,  0.10),
    vec3<f32>(-0.08,  0.11,  0.08),
    vec3<f32>( 0.10, -0.09,  0.11),
    vec3<f32>(-0.11, -0.07,  0.09),
    vec3<f32>( 0.06,  0.14,  0.12),
    vec3<f32>(-0.13,  0.05,  0.10),
    vec3<f32>( 0.09, -0.12,  0.13),
    vec3<f32>(-0.07, -0.14,  0.11),
    // Outer samples (farther, lower weight)
    vec3<f32>( 0.18,  0.08,  0.15),
    vec3<f32>(-0.12,  0.17,  0.14),
    vec3<f32>( 0.15, -0.14,  0.16),
    vec3<f32>(-0.17, -0.10,  0.13),
    vec3<f32>( 0.10,  0.20,  0.18),
    vec3<f32>(-0.19,  0.09,  0.15),
    vec3<f32>( 0.14, -0.18,  0.17),
    vec3<f32>(-0.11, -0.19,  0.16),
    // Outermost samples
    vec3<f32>( 0.24,  0.12,  0.20),
    vec3<f32>(-0.16,  0.23,  0.19),
    vec3<f32>( 0.21, -0.18,  0.22),
    vec3<f32>(-0.23, -0.14,  0.18),
    vec3<f32>( 0.14,  0.26,  0.24),
    vec3<f32>(-0.25,  0.13,  0.21),
    vec3<f32>( 0.19, -0.24,  0.23),
    vec3<f32>(-0.15, -0.25,  0.22),
);

@group(0) @binding(0) var depth_texture: texture_depth_2d;
@group(0) @binding(1) var normal_texture: texture_2d<f32>;
@group(0) @binding(2) var noise_texture: texture_2d<f32>;
@group(0) @binding(3) var tex_sampler: sampler;
@group(0) @binding(4) var<uniform> uniforms: SsaoUniforms;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var out: VertexOutput;
    let x = f32((vertex_index & 1u) << 2u) - 1.0;
    let y = f32((vertex_index & 2u) << 1u) - 1.0;
    out.position = vec4<f32>(x, y, 0.0, 1.0);
    out.uv = vec2<f32>((x + 1.0) * 0.5, (1.0 - y) * 0.5);
    return out;
}

// Reconstruct view-space position from depth
fn view_pos_from_depth(uv: vec2<f32>, depth: f32) -> vec3<f32> {
    let ndc = vec4<f32>(uv * 2.0 - 1.0, depth, 1.0);
    let view = uniforms.inv_proj * ndc;
    return view.xyz / view.w;
}

// Decode normal from texture sample
fn decode_normal(sample: vec4<f32>) -> vec3<f32> {
    return normalize(sample.xyz * 2.0 - 1.0);
}

// Detect geometric edges by checking normal discontinuity in neighborhood
fn compute_edge_factor(uv: vec2<f32>, center_normal: vec3<f32>) -> f32 {
    let texel = vec2<f32>(1.0 / uniforms.screen_width, 1.0 / uniforms.screen_height);

    // Sample normals in a cross pattern
    let n_left = textureSample(normal_texture, tex_sampler, uv + vec2<f32>(-texel.x, 0.0));
    let n_right = textureSample(normal_texture, tex_sampler, uv + vec2<f32>(texel.x, 0.0));
    let n_up = textureSample(normal_texture, tex_sampler, uv + vec2<f32>(0.0, -texel.y));
    let n_down = textureSample(normal_texture, tex_sampler, uv + vec2<f32>(0.0, texel.y));

    // Compute how much the normal changes across neighboring pixels
    var max_dot = 1.0;
    if (n_left.a > 0.5) {
        max_dot = min(max_dot, dot(center_normal, decode_normal(n_left)));
    }
    if (n_right.a > 0.5) {
        max_dot = min(max_dot, dot(center_normal, decode_normal(n_right)));
    }
    if (n_up.a > 0.5) {
        max_dot = min(max_dot, dot(center_normal, decode_normal(n_up)));
    }
    if (n_down.a > 0.5) {
        max_dot = min(max_dot, dot(center_normal, decode_normal(n_down)));
    }

    // max_dot = 1.0 means smooth surface, max_dot < 0.7 means sharp edge (>45 degrees)
    // Return factor: 1.0 for smooth areas, reduced for edges
    // Threshold at ~60 degrees (cos(60) = 0.5)
    return smoothstep(0.3, 0.8, max_dot);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let depth = textureSample(depth_texture, tex_sampler, in.uv);
    if (depth >= 1.0) {
        return vec4<f32>(1.0);
    }

    let normal_sample = textureSample(normal_texture, tex_sampler, in.uv);
    if (normal_sample.a < 0.5) {
        return vec4<f32>(1.0);
    }

    // Normal in view space (decode from [0,1] to [-1,1])
    let normal = decode_normal(normal_sample);

    // Detect if we're at a geometric edge (sharp corner/edge of cube, etc.)
    let edge_factor = compute_edge_factor(in.uv, normal);

    // Fragment position in view space
    let frag_pos = view_pos_from_depth(in.uv, depth);

    // Random rotation from noise texture
    let noise_scale = vec2<f32>(uniforms.screen_width / 4.0, uniforms.screen_height / 4.0);
    let random_vec = textureSample(noise_texture, tex_sampler, in.uv * noise_scale).xyz * 2.0 - 1.0;

    // Gram-Schmidt to create orthonormal TBN basis
    let tangent = normalize(random_vec - normal * dot(random_vec, normal));
    let bitangent = cross(normal, tangent);
    let tbn = mat3x3<f32>(tangent, bitangent, normal);

    // Accumulate occlusion
    var occlusion = 0.0;
    let sample_count = min(uniforms.sample_count, KERNEL_SIZE);
    let radius = uniforms.radius;

    for (var i = 0u; i < sample_count; i++) {
        // Sample position in view space
        let sample_offset = tbn * kernel[i];
        let sample_pos = frag_pos + sample_offset * radius;

        // Project to screen space
        let proj_sample = uniforms.proj * vec4<f32>(sample_pos, 1.0);
        var sample_uv = (proj_sample.xy / proj_sample.w) * 0.5 + 0.5;
        sample_uv.y = 1.0 - sample_uv.y;

        // Bounds check
        if (sample_uv.x < 0.0 || sample_uv.x > 1.0 || sample_uv.y < 0.0 || sample_uv.y > 1.0) {
            continue;
        }

        // Get actual depth at sample location
        let sample_depth = textureSample(depth_texture, tex_sampler, sample_uv);
        if (sample_depth >= 1.0) {
            continue;
        }

        // Get the normal at the sample location for edge-aware weighting
        let sample_normal_tex = textureSample(normal_texture, tex_sampler, sample_uv);

        let sample_view_pos = view_pos_from_depth(sample_uv, sample_depth);

        // Range check: is the actual surface within our sampling radius?
        let depth_diff = abs(frag_pos.z - sample_view_pos.z);
        let range_check = smoothstep(0.0, 1.0, radius / (depth_diff + 0.001));

        // Normal-based weight: reduce contribution when sampling across different surfaces
        // This prevents dark halos at edges where samples hit the adjacent face
        var normal_weight = 1.0;
        if (sample_normal_tex.a > 0.5) {
            let sample_normal = decode_normal(sample_normal_tex);
            let ndot = dot(normal, sample_normal);
            // If normals differ significantly (different faces), reduce contribution
            // cos(60°) = 0.5, cos(90°) = 0
            normal_weight = smoothstep(-0.2, 0.7, ndot);
        }

        // Occlusion check: is the actual surface closer than expected?
        let expected_z = sample_pos.z;
        let actual_z = sample_view_pos.z;

        // If actual surface is closer (less negative / more positive) than expected, it occludes
        if (actual_z >= expected_z + uniforms.bias) {
            occlusion += range_check * normal_weight;
        }
    }

    // Normalize
    occlusion = occlusion / f32(sample_count);

    // Apply edge factor to reduce AO at sharp geometric edges
    // This prevents the concentrated black areas at cube corners
    occlusion *= edge_factor;

    // Convert to visibility (1 = fully visible, 0 = fully occluded)
    var visibility = 1.0 - occlusion;

    // Apply intensity - higher intensity = darker shadows
    visibility = pow(visibility, uniforms.intensity);

    return vec4<f32>(visibility, visibility, visibility, 1.0);
}
