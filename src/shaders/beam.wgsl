// Beam shader - photon lance rendering with glow.

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
    @location(0) start_pos: vec2<f32>,
    @location(1) end_pos: vec2<f32>,
    @location(2) width: f32,
    @location(3) color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) frag_color: vec4<f32>,
    @location(1) edge_dist: f32, // -1 to 1 across the beam width
    @location(2) along_dist: f32, // 0 to 1 along beam length
};

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_index: u32,
    instance: InstanceInput,
) -> VertexOutput {
    var out: VertexOutput;

    let dir = instance.end_pos - instance.start_pos;
    let len = length(dir);
    let forward = dir / max(len, 0.0001);
    let right = vec2<f32>(forward.y, -forward.x); // perpendicular

    let half_w = instance.width * 0.5;

    // Quad along the line: 6 vertices, 2 triangles.
    // Along: 0 = start, 1 = end. Across: -1 = left, 1 = right.
    var along: f32;
    var across: f32;
    switch (vertex_index) {
        case 0u: { along = 0.0; across = -1.0; }
        case 1u: { along = 1.0; across = -1.0; }
        case 2u: { along = 1.0; across = 1.0; }
        case 3u: { along = 0.0; across = -1.0; }
        case 4u: { along = 1.0; across = 1.0; }
        case 5u: { along = 0.0; across = 1.0; }
        default: { along = 0.0; across = 0.0; }
    }

    let world_pos = instance.start_pos
        + forward * (along * len)
        + right * (across * half_w);

    out.clip_position = camera.view_proj * vec4<f32>(world_pos, 0.0, 1.0);
    out.frag_color = instance.color;
    out.edge_dist = across;
    out.along_dist = along;

    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Glow effect: bright center, falloff toward edges.
    let abs_edge = abs(in.edge_dist);

    // Core beam: very bright narrow center.
    let core = exp(-abs_edge * abs_edge * 16.0);
    // Outer glow: softer, wider.
    let glow = exp(-abs_edge * abs_edge * 4.0) * 0.5;

    let intensity = core + glow;

    // Slight fade at beam endpoints for soft termination.
    let end_fade = smoothstep(0.0, 0.05, in.along_dist) * smoothstep(1.0, 0.95, in.along_dist);

    let final_intensity = intensity * end_fade;

    // Output is additive-blend ready (premultiplied alpha).
    let rgb = in.frag_color.rgb * final_intensity;
    let alpha = in.frag_color.a * final_intensity;

    return vec4<f32>(rgb, alpha);
}
