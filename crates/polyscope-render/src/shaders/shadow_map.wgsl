// Shadow map generation shader
// Renders scene depth from light's perspective

struct LightUniforms {
    view_proj: mat4x4<f32>,
    light_dir: vec4<f32>,
}

struct ModelUniforms {
    model: mat4x4<f32>,
}

@group(0) @binding(0) var<uniform> light: LightUniforms;
@group(0) @binding(1) var<uniform> model: ModelUniforms;
@group(0) @binding(2) var<storage, read> positions: array<vec4<f32>>;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var out: VertexOutput;

    let world_pos = model.model * positions[vertex_index];
    out.position = light.view_proj * world_pos;

    return out;
}

// Fragment shader just writes depth (no color output needed for shadow map)
@fragment
fn fs_main(in: VertexOutput) {
    // Depth is automatically written
}
