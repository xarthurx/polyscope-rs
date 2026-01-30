// Tone mapping post-processing shader
// Applies exposure, Reinhard tone mapping, gamma correction, and optional SSAO

struct ToneMapUniforms {
    exposure: f32,
    white_level: f32,
    gamma: f32,
    ssao_enabled: u32, // 0 = disabled, 1 = enabled
    _padding: vec4<f32>, // Pad to 32 bytes (workaround for wgpu late binding size validation)
}

@group(0) @binding(0) var input_texture: texture_2d<f32>;
@group(0) @binding(1) var input_sampler: sampler;
@group(0) @binding(2) var<uniform> uniforms: ToneMapUniforms;
@group(0) @binding(3) var ssao_texture: texture_2d<f32>;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

// Fullscreen triangle
@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var out: VertexOutput;

    // Generate fullscreen triangle (covers [-1,1] x [-1,1])
    let x = f32((vertex_index & 1u) << 2u) - 1.0;
    let y = f32((vertex_index & 2u) << 1u) - 1.0;

    out.position = vec4<f32>(x, y, 0.0, 1.0);
    // Convert from clip space [-1,1] to UV space [0,1]
    // Note: Y is flipped for texture sampling
    out.uv = vec2<f32>((x + 1.0) * 0.5, (1.0 - y) * 0.5);

    return out;
}

// Reinhard tone mapping with white point
fn reinhard_extended(color: vec3<f32>, white: f32) -> vec3<f32> {
    let white_sq = white * white;
    let numerator = color * (1.0 + color / white_sq);
    return numerator / (1.0 + color);
}

// Gamma correction
fn gamma_correct(color: vec3<f32>, gamma: f32) -> vec3<f32> {
    return pow(color, vec3<f32>(1.0 / gamma));
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Sample the HDR input
    let hdr_color = textureSample(input_texture, input_sampler, in.uv);

    // Apply exposure
    var color = hdr_color.rgb * uniforms.exposure;

    // Apply SSAO if enabled (multiply before tone mapping for correct darkening)
    if (uniforms.ssao_enabled == 1u) {
        let ssao = textureSample(ssao_texture, input_sampler, in.uv).r;
        color = color * ssao;
    }

    // Apply Reinhard tone mapping with white point
    color = reinhard_extended(color, uniforms.white_level);

    // Apply gamma correction
    color = gamma_correct(color, uniforms.gamma);

    // Clamp to valid range
    color = clamp(color, vec3<f32>(0.0), vec3<f32>(1.0));

    return vec4<f32>(color, hdr_color.a);
}
