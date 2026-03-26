// Sprite shader - instanced chevron rendering for ships and projectiles.

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
    @location(1) rotation: f32,
    @location(2) scale: f32,
    @location(3) color: vec4<f32>,
    @location(4) shield_alpha: f32,
    @location(5) thrust_type: u32,
    @location(6) thrust_magnitude: f32,
    @location(7) turret_angle: f32,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) frag_color: vec4<f32>,
    @location(1) local_pos: vec2<f32>,
    @location(2) shield_alpha: f32,
    @location(3) part_id: f32, // 0 = hull, 1 = thrust, 2 = turret
};

fn rotate2d(p: vec2<f32>, angle: f32) -> vec2<f32> {
    let c = cos(angle);
    let s = sin(angle);
    return vec2<f32>(p.x * c - p.y * s, p.x * s + p.y * c);
}

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_index: u32,
    instance: InstanceInput,
) -> VertexOutput {
    var out: VertexOutput;
    var local: vec2<f32>;
    var col: vec4<f32> = instance.color;
    var part: f32 = 0.0;
    var rot_angle: f32 = instance.rotation;

    if (vertex_index < 3u) {
        // Hull chevron: a pointed triangle facing +Y (forward).
        switch (vertex_index) {
            case 0u: { local = vec2<f32>(0.0, 0.5); }   // nose
            case 1u: { local = vec2<f32>(-0.35, -0.4); } // left wing
            case 2u: { local = vec2<f32>(0.35, -0.4); }  // right wing
            default: { local = vec2<f32>(0.0, 0.0); }
        }
        part = 0.0;
    } else if (vertex_index < 6u) {
        // Thrust flame triangle behind the ship.
        let tidx = vertex_index - 3u;
        let mag = instance.thrust_magnitude;
        switch (tidx) {
            case 0u: { local = vec2<f32>(-0.15, -0.4); }
            case 1u: { local = vec2<f32>(0.15, -0.4); }
            case 2u: { local = vec2<f32>(0.0, -0.4 - 0.5 * mag); } // extends with thrust
            default: { local = vec2<f32>(0.0, 0.0); }
        }
        // Tint based on thrust type.
        if (instance.thrust_type == 0u) {
            col = vec4<f32>(1.0, 0.6, 0.1, mag); // orange chemical
        } else {
            col = vec4<f32>(0.3, 0.5, 1.0, mag); // blue ion
        }
        part = 1.0;
    } else {
        // Turret line (thin triangle pointing outward).
        let tidx = vertex_index - 6u;
        // Turret rotates independently via turret_angle.
        rot_angle = instance.rotation + instance.turret_angle;
        switch (tidx) {
            case 0u: { local = vec2<f32>(-0.03, 0.0); }
            case 1u: { local = vec2<f32>(0.03, 0.0); }
            case 2u: { local = vec2<f32>(0.0, 0.45); } // barrel tip
            default: { local = vec2<f32>(0.0, 0.0); }
        }
        col = vec4<f32>(0.8, 0.8, 0.8, 1.0);
        part = 2.0;
    }

    // Transform: scale, rotate, translate.
    let scaled = local * instance.scale;
    let rotated = rotate2d(scaled, rot_angle);
    let world_pos = rotated + instance.position;

    out.clip_position = camera.view_proj * vec4<f32>(world_pos, 0.0, 1.0);
    out.frag_color = col;
    out.local_pos = local;
    out.shield_alpha = instance.shield_alpha;
    out.part_id = part;

    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    var color = in.frag_color;

    // For thrust flame, use the alpha as intensity (faded when no thrust).
    if (in.part_id > 0.5 && in.part_id < 1.5) {
        if (color.a < 0.01) {
            discard;
        }
    }

    // Shield overlay: add a translucent circle on top of hull vertices.
    if (in.part_id < 0.5 && in.shield_alpha > 0.0) {
        let dist = length(in.local_pos);
        let shield_ring = smoothstep(0.55, 0.5, dist) * smoothstep(0.35, 0.4, dist);
        let shield_fill = smoothstep(0.55, 0.3, dist) * 0.15;
        let shield_intensity = (shield_ring * 0.8 + shield_fill) * in.shield_alpha;
        let shield_color = vec4<f32>(0.3, 0.6, 1.0, shield_intensity);
        color = vec4<f32>(
            color.rgb + shield_color.rgb * shield_color.a,
            max(color.a, shield_color.a)
        );
    }

    return color;
}
