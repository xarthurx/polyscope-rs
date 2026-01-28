// Depth-to-mask shader for planar shadows
// Converts a depth buffer to a shadow mask:
// - White (1.0) where no geometry (depth = 1.0, cleared value)
// - Black (0.0) where geometry was rendered (shadow)

@group(0) @binding(0) var depth_texture: texture_depth_2d;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

// Fullscreen triangle
@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var out: VertexOutput;

    // Generate fullscreen triangle vertices
    let x = f32((vertex_index & 1u) << 2u) - 1.0;
    let y = f32((vertex_index & 2u) << 1u) - 1.0;

    out.position = vec4<f32>(x, y, 0.0, 1.0);
    out.uv = vec2<f32>((x + 1.0) * 0.5, (1.0 - y) * 0.5);

    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) f32 {
    // Get texture dimensions
    let tex_dims = textureDimensions(depth_texture);

    // Convert UV to texel coordinates
    let texel_coord = vec2<i32>(in.uv * vec2<f32>(tex_dims));

    // Load depth directly (no sampling needed)
    let depth = textureLoad(depth_texture, texel_coord, 0);

    // Convert depth to shadow mask:
    // - depth near 1.0 means no geometry (clear value) -> output 0.0 (no shadow)
    // - depth < 1.0 means geometry was rendered -> output 1.0 (shadow)
    let shadow = 1.0 - step(0.9999, depth);

    return shadow;
}
