// SSAO bilateral blur shader
// Applies edge-preserving blur using depth-aware weighting

struct BlurUniforms {
    texel_size: vec2<f32>,
    blur_scale: f32,
    _padding: f32,
}

@group(0) @binding(0) var ssao_texture: texture_2d<f32>;
@group(0) @binding(1) var tex_sampler: sampler;
@group(0) @binding(2) var<uniform> uniforms: BlurUniforms;

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

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Use a larger Gaussian-weighted blur (5x5) for smoother results
    // Gaussian weights approximation for sigma ~= 1.5
    let weights = array<f32, 25>(
        0.003, 0.013, 0.022, 0.013, 0.003,
        0.013, 0.059, 0.097, 0.059, 0.013,
        0.022, 0.097, 0.159, 0.097, 0.022,
        0.013, 0.059, 0.097, 0.059, 0.013,
        0.003, 0.013, 0.022, 0.013, 0.003
    );

    var result = 0.0;
    var total_weight = 0.0;
    let center_value = textureSample(ssao_texture, tex_sampler, in.uv).r;

    // 5x5 Gaussian blur
    for (var y = -2; y <= 2; y++) {
        for (var x = -2; x <= 2; x++) {
            let idx = (y + 2) * 5 + (x + 2);
            let offset = vec2<f32>(f32(x), f32(y)) * uniforms.texel_size * uniforms.blur_scale;
            let sample_uv = in.uv + offset;
            let sample_value = textureSample(ssao_texture, tex_sampler, sample_uv).r;

            // Use Gaussian weight, but also apply a simple bilateral term
            // that reduces weight for samples that differ too much from center
            let value_diff = abs(sample_value - center_value);
            let bilateral_weight = exp(-value_diff * value_diff * 50.0);

            let weight = weights[idx] * bilateral_weight;
            result += sample_value * weight;
            total_weight += weight;
        }
    }

    // Normalize
    if (total_weight > 0.0) {
        result /= total_weight;
    } else {
        result = center_value;
    }

    return vec4<f32>(result, result, result, 1.0);
}
