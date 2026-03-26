// Trail shader - orbital trail line rendering with fade.

struct CameraUniform {
    view_proj: mat4x4<f32>,
    camera_pos: vec2<f32>,
    zoom: f32,
    _pad0: f32,
    viewport_size: vec2<f32>,
    _pad1: vec2<f32>,
};
@group(0) @binding(0) var<uniform> camera: CameraUniform;

struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) alpha: f32,
    @location(2) color: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) frag_color: vec3<f32>,
    @location(1) frag_alpha: f32,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = camera.view_proj * vec4<f32>(in.position, 0.0, 1.0);
    out.frag_color = in.color;
    out.frag_alpha = in.alpha;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(in.frag_color * in.frag_alpha, in.frag_alpha);
}
