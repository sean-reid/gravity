// Black hole shader - event horizon, photon sphere, accretion disk.

struct CameraUniform {
    view_proj: mat4x4<f32>,
    camera_pos: vec2<f32>,
    zoom: f32,
    _pad0: f32,
    viewport_size: vec2<f32>,
    _pad1: vec2<f32>,
};
@group(0) @binding(0) var<uniform> camera: CameraUniform;

struct BlackHoleUniform {
    bh_position: vec2<f32>,
    bh_radius: f32,
    time: f32,
};
@group(1) @binding(0) var<uniform> bh: BlackHoleUniform;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_pos: vec2<f32>,
};

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_index: u32,
) -> VertexOutput {
    var out: VertexOutput;

    // Screen-aligned quad covering 4x radius around the BH center.
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

    let extent = bh.bh_radius * 4.0;
    let world_pos = bh.bh_position + corner * extent;

    out.clip_position = camera.view_proj * vec4<f32>(world_pos, 0.0, 1.0);
    out.world_pos = world_pos;

    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let delta = in.world_pos - bh.bh_position;
    let dist = length(delta);
    let r_s = bh.bh_radius; // Schwarzschild radius

    let r_norm = dist / r_s; // normalized distance in units of r_s

    // Outside 4.0 r_s: transparent.
    if (r_norm > 4.0) {
        discard;
    }

    // Inside r_s: event horizon - pure black void.
    if (r_norm < 1.0) {
        return vec4<f32>(0.0, 0.0, 0.0, 1.0);
    }

    // Angular coordinate for rotation effects.
    let angle = atan2(delta.y, delta.x);
    let rotating_angle = angle - bh.time * 0.8; // accretion disk rotation

    // Photon sphere glow: 1.0 - 1.5 r_s.
    var photon_glow = 0.0;
    if (r_norm < 1.5) {
        let photon_t = (r_norm - 1.0) / 0.5; // 0 at r_s, 1 at 1.5 r_s
        // Bright at the inner edge, fading outward.
        photon_glow = exp(-photon_t * 3.0) * 0.6;
    }

    // Accretion disk: 1.0 - 4.0 r_s.
    var disk_color = vec3<f32>(0.0);
    var disk_alpha = 0.0;

    if (r_norm >= 1.0 && r_norm <= 4.0) {
        let disk_t = (r_norm - 1.0) / 3.0; // 0 at inner edge, 1 at outer

        // Radial color gradient: white -> yellow -> orange -> dim red.
        var radial_color: vec3<f32>;
        if (disk_t < 0.2) {
            radial_color = mix(vec3<f32>(1.0, 1.0, 1.0), vec3<f32>(1.0, 0.9, 0.5), disk_t / 0.2);
        } else if (disk_t < 0.5) {
            radial_color = mix(vec3<f32>(1.0, 0.9, 0.5), vec3<f32>(1.0, 0.5, 0.1), (disk_t - 0.2) / 0.3);
        } else {
            radial_color = mix(vec3<f32>(1.0, 0.5, 0.1), vec3<f32>(0.4, 0.1, 0.05), (disk_t - 0.5) / 0.5);
        }

        // Radial intensity falloff: brighter near the center.
        let radial_intensity = (1.0 - disk_t) * (1.0 - disk_t);

        // Relativistic beaming: one side brighter than the other.
        // The approaching side (based on rotation) is brighter.
        let beaming = 0.7 + 0.3 * cos(rotating_angle);

        // Angular structure: slight brightness variation for visual interest.
        let angular_detail = 0.85 + 0.15 * sin(rotating_angle * 3.0 + bh.time * 1.5);

        disk_color = radial_color * radial_intensity * beaming * angular_detail;
        disk_alpha = radial_intensity * 0.9;

        // Soft fade at outer edge.
        let outer_fade = smoothstep(4.0, 3.5, r_norm);
        disk_alpha *= outer_fade;
    }

    // Combine photon sphere and accretion disk.
    let photon_color = vec3<f32>(0.7, 0.8, 1.0) * photon_glow;
    let final_color = disk_color + photon_color;
    let final_alpha = max(disk_alpha, photon_glow);

    return vec4<f32>(final_color, final_alpha);
}
