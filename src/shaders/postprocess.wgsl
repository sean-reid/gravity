// Post-processing shader - gravitational lensing, bloom, vignette.

const MAX_BLACK_HOLES: u32 = 8u;

struct PostProcessUniform {
    viewport_size: vec2<f32>,
    depth_factor: f32,
    num_black_holes: u32,
    // Each BH: vec4(pos.x, pos.y, radius, 0)
    black_holes: array<vec4<f32>, 8>,
};
@group(0) @binding(0) var<uniform> params: PostProcessUniform;
@group(0) @binding(1) var scene_texture: texture_2d<f32>;
@group(0) @binding(2) var scene_sampler: sampler;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var out: VertexOutput;

    // Full-screen triangle (3 vertices cover the entire screen).
    var pos: vec2<f32>;
    var uv: vec2<f32>;
    switch (vertex_index) {
        case 0u: { pos = vec2<f32>(-1.0, -1.0); uv = vec2<f32>(0.0, 1.0); }
        case 1u: { pos = vec2<f32>(3.0, -1.0);  uv = vec2<f32>(2.0, 1.0); }
        case 2u: { pos = vec2<f32>(-1.0, 3.0);  uv = vec2<f32>(0.0, -1.0); }
        default: { pos = vec2<f32>(0.0, 0.0);   uv = vec2<f32>(0.0, 0.0); }
    }

    out.clip_position = vec4<f32>(pos, 0.0, 1.0);
    out.uv = uv;

    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    var uv = in.uv;

    // --- Gravitational lensing ---
    // For each black hole, displace UV outward from the BH center.
    for (var i = 0u; i < params.num_black_holes; i = i + 1u) {
        let bh_data = params.black_holes[i];
        let bh_screen_pos = bh_data.xy; // in UV space (0..1)
        let bh_radius_uv = bh_data.z;   // radius in UV units

        let to_frag = uv - bh_screen_pos;
        let dist = length(to_frag);

        if (dist > 0.001) {
            let dir = to_frag / dist;
            // Einstein ring-style displacement: strength ~ r_s / distance.
            let r_s = bh_radius_uv;
            let strength = r_s * r_s / (dist + r_s * 0.5);
            // Push UV outward from BH center (light bends toward mass).
            uv = uv + dir * strength * 0.3;
        }
    }

    // Clamp UV to valid range.
    uv = clamp(uv, vec2<f32>(0.0), vec2<f32>(1.0));

    // Sample the scene.
    var color = textureSample(scene_texture, scene_sampler, uv);

    // --- Simple bloom approximation ---
    // Sample surrounding pixels and add bright ones back in.
    let texel = 1.0 / params.viewport_size;
    var bloom = vec3<f32>(0.0);
    let bloom_samples = 8;
    let bloom_radius = 3.0;

    // Radial sample offsets (8 directions).
    let offsets = array<vec2<f32>, 8>(
        vec2<f32>(1.0, 0.0),
        vec2<f32>(0.707, 0.707),
        vec2<f32>(0.0, 1.0),
        vec2<f32>(-0.707, 0.707),
        vec2<f32>(-1.0, 0.0),
        vec2<f32>(-0.707, -0.707),
        vec2<f32>(0.0, -1.0),
        vec2<f32>(0.707, -0.707),
    );

    for (var i = 0; i < 8; i = i + 1) {
        let sample_uv = uv + offsets[i] * texel * bloom_radius;
        let s = textureSample(scene_texture, scene_sampler, sample_uv).rgb;
        // Only accumulate bright pixels (threshold).
        let luminance = dot(s, vec3<f32>(0.299, 0.587, 0.114));
        let bright = max(luminance - 0.7, 0.0) / 0.3;
        bloom += s * bright;
    }
    bloom /= 8.0;
    color = vec4<f32>(color.rgb + bloom * 0.4, color.a);

    // --- Vignette ---
    let center = vec2<f32>(0.5, 0.5);
    let vig_dist = length(in.uv - center) * 1.4; // normalize so corners ~ 1.0
    let vignette = 1.0 - smoothstep(0.4, 1.2, vig_dist) * params.depth_factor;
    color = vec4<f32>(color.rgb * vignette, color.a);

    return color;
}
