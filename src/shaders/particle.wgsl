// Particle shader - GPU billboard quads with circular falloff.

struct CameraUniform {
    view_proj: mat4x4<f32>,
    camera_pos: vec2<f32>,
    zoom: f32,
    _pad0: f32,
    viewport_size: vec2<f32>,
    _pad1: vec2<f32>,
};
@group(0) @binding(0) var<uniform> camera: CameraUniform;

struct InstanceInput {
    @location(0) position: vec2<f32>,
    @location(1) velocity: vec2<f32>,
    @location(2) size: f32,
    @location(3) color: vec4<f32>,
    @location(4) age: f32,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) frag_color: vec4<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) age: f32,
};

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_index: u32,
    instance: InstanceInput,
) -> VertexOutput {
    var out: VertexOutput;

    // Generate a quad from 6 vertices (two triangles).
    // Quad corners: (-1,-1), (1,-1), (1,1), (-1,1)
    // Triangle 1: 0,1,2  Triangle 2: 0,2,3
    var corner: vec2<f32>;
    switch (vertex_index) {
        case 0u: { corner = vec2<f32>(-1.0, -1.0); }
        case 1u: { corner = vec2<f32>(1.0, -1.0); }
        case 2u: { corner = vec2<f32>(1.0, 1.0); }
        case 3u: { corner = vec2<f32>(-1.0, -1.0); }
        case 4u: { corner = vec2<f32>(1.0, 1.0); }
        case 5u: { corner = vec2<f32>(-1.0, 1.0); }
        default: { corner = vec2<f32>(0.0, 0.0); }
    }

    let half_size = instance.size * 0.5;
    let world_pos = instance.position + corner * half_size;

    out.clip_position = camera.view_proj * vec4<f32>(world_pos, 0.0, 1.0);
    out.frag_color = instance.color;
    out.uv = corner; // ranges from -1 to 1
    out.age = instance.age;

    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let dist = length(in.uv);

    // Discard outside the circle.
    if (dist > 1.0) {
        discard;
    }

    // Circular falloff: bright center, fading toward edges.
    let falloff = 1.0 - smoothstep(0.0, 1.0, dist);

    // Age-based alpha fade: particles fade out as they age (0.0 = new, 1.0 = dead).
    let age_alpha = 1.0 - clamp(in.age, 0.0, 1.0);

    let final_alpha = in.frag_color.a * falloff * age_alpha;

    return vec4<f32>(in.frag_color.rgb, final_alpha);
}
