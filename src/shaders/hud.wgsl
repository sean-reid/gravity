// HUD shader - screen-space UI elements with texture atlas and color tinting.

struct HudUniform {
    screen_proj: mat4x4<f32>,
    tint_color: vec4<f32>,
    use_texture: u32, // 1 = sample texture, 0 = solid color from vertex
    _pad0: u32,
    _pad1: u32,
    _pad2: u32,
};
@group(0) @binding(0) var<uniform> hud: HudUniform;
@group(0) @binding(1) var atlas_texture: texture_2d<f32>;
@group(0) @binding(2) var atlas_sampler: sampler;

struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = hud.screen_proj * vec4<f32>(in.position, 0.0, 1.0);
    out.uv = in.uv;
    out.color = in.color;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    if (hud.use_texture == 1u) {
        // Text: sample texture, multiply by vertex color for tinting
        let tex_color = textureSample(atlas_texture, atlas_sampler, in.uv);
        return tex_color * in.color;
    } else {
        // Solid rects: use vertex color directly
        return in.color;
    }
}
