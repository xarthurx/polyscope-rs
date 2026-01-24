// Shadow blur shader (separable Gaussian blur)
// Used to soften shadow edges

struct BlurUniforms {
    direction: vec2<f32>,  // (1,0) for horizontal, (0,1) for vertical
    texel_size: vec2<f32>, // 1.0 / texture_size
}

@group(0) @binding(0) var input_texture: texture_2d<f32>;
@group(0) @binding(1) var input_sampler: sampler;
@group(0) @binding(2) var<uniform> uniforms: BlurUniforms;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

// Fullscreen triangle
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
    // 5-tap Gaussian blur weights
    let weights = array<f32, 5>(0.227027, 0.1945946, 0.1216216, 0.054054, 0.016216);

    var result = textureSample(input_texture, input_sampler, in.uv) * weights[0];

    let offset = uniforms.direction * uniforms.texel_size;

    for (var i = 1; i < 5; i++) {
        let sample_offset = offset * f32(i);
        result += textureSample(input_texture, input_sampler, in.uv + sample_offset) * weights[i];
        result += textureSample(input_texture, input_sampler, in.uv - sample_offset) * weights[i];
    }

    return result;
}
