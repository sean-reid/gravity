// Starfield shader - background parallax stars with twinkle.

struct CameraUniform {
    view_proj: mat4x4<f32>,
    camera_pos: vec2<f32>,
    zoom: f32,
    _pad0: f32,
    viewport_size: vec2<f32>,
    _pad1: vec2<f32>,
};
@group(0) @binding(0) var<uniform> camera: CameraUniform;

struct StarfieldUniform {
    time: f32,
    _pad0: f32,
    _pad1: f32,
    _pad2: f32,
};
@group(1) @binding(0) var<uniform> starfield: StarfieldUniform;

struct InstanceInput {
    @location(0) position: vec2<f32>,
    @location(1) brightness: f32,
    @location(2) parallax_factor: f32,
    @location(3) phase: f32,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) brightness: f32,
    @location(1) uv: vec2<f32>,
    @location(2) phase: f32,
};

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_index: u32,
    instance: InstanceInput,
) -> VertexOutput {
    var out: VertexOutput;

    // Small point quad: 6 vertices for two triangles.
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

    // Star size in world units. Slightly larger for brighter stars.
    let star_size = (0.8 + instance.brightness * 0.7) / camera.zoom;

    // Parallax offset: stars closer to camera (higher parallax) move more with camera.
    let parallax_offset = camera.camera_pos * instance.parallax_factor;
    let world_pos = (instance.position - parallax_offset) + corner * star_size;

    out.clip_position = camera.view_proj * vec4<f32>(world_pos, 0.0, 1.0);
    out.brightness = instance.brightness;
    out.uv = corner;
    out.phase = instance.phase;

    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let dist = length(in.uv);

    // Discard outside the point.
    if (dist > 1.0) {
        discard;
    }

    // Soft circular falloff.
    let falloff = 1.0 - smoothstep(0.0, 1.0, dist);

    // Twinkle: modulate brightness with sin wave.
    let twinkle = sin(starfield.time * 2.0 + in.phase) * 0.15 + 0.85;

    let intensity = in.brightness * twinkle * falloff;

    // Stars are white with slight warmth for brighter ones.
    let warm = vec3<f32>(1.0, 0.95, 0.9);
    let cool = vec3<f32>(0.9, 0.95, 1.0);
    let star_color = mix(cool, warm, in.brightness);

    return vec4<f32>(star_color * intensity, intensity);
}
