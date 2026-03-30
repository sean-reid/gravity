use wgpu::util::DeviceExt;

/// Per-star instance data sent to the GPU.
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct StarInstance {
    pub position: [f32; 2],
    pub brightness: f32,
    pub parallax_factor: f32,
    pub phase: f32,
    pub _pad: [f32; 3],
}

/// Time uniform for twinkle animation.
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct StarfieldUniform {
    time: f32,
    _pad0: f32,
    _pad1: f32,
    _pad2: f32,
}

const MAX_STARS: u32 = 4096;

pub struct StarfieldPipeline {
    pipeline: wgpu::RenderPipeline,
    instance_buffer: wgpu::Buffer,
    time_buffer: wgpu::Buffer,
    time_bind_group: wgpu::BindGroup,
    star_count: u32,
}

impl StarfieldPipeline {
    pub fn new(
        device: &wgpu::Device,
        camera_bind_group_layout: &wgpu::BindGroupLayout,
        format: wgpu::TextureFormat,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("starfield_shader"),
            source: wgpu::ShaderSource::Wgsl(
                include_str!("../../shaders/starfield.wgsl").into(),
            ),
        });

        let time_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("starfield_time_buffer"),
            contents: bytemuck::cast_slice(&[StarfieldUniform {
                time: 0.0,
                _pad0: 0.0,
                _pad1: 0.0,
                _pad2: 0.0,
            }]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let time_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("starfield_time_bgl"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let time_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("starfield_time_bg"),
            layout: &time_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: time_buffer.as_entire_binding(),
            }],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("starfield_pipeline_layout"),
            bind_group_layouts: &[camera_bind_group_layout, &time_bind_group_layout],
            push_constant_ranges: &[],
        });

        let instance_buffer_layout = wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<StarInstance>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                // position: vec2<f32>
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x2,
                    offset: 0,
                    shader_location: 0,
                },
                // brightness: f32
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32,
                    offset: 8,
                    shader_location: 1,
                },
                // parallax_factor: f32
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32,
                    offset: 12,
                    shader_location: 2,
                },
                // phase: f32
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32,
                    offset: 16,
                    shader_location: 3,
                },
            ],
        };

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("starfield_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[instance_buffer_layout],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        let instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("starfield_instance_buffer"),
            size: (std::mem::size_of::<StarInstance>() as u64) * MAX_STARS as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            pipeline,
            instance_buffer,
            time_buffer,
            time_bind_group,
            star_count: 0,
        }
    }

    pub fn update_stars(&mut self, queue: &wgpu::Queue, stars: &[StarInstance]) {
        let count = stars.len().min(MAX_STARS as usize);
        self.star_count = count as u32;
        if count > 0 {
            queue.write_buffer(
                &self.instance_buffer,
                0,
                bytemuck::cast_slice(&stars[..count]),
            );
        }
    }

    pub fn render<'a>(
        &'a self,
        render_pass: &mut wgpu::RenderPass<'a>,
        camera_bind_group: &'a wgpu::BindGroup,
        time: f32,
        queue: &wgpu::Queue,
    ) {
        if self.star_count == 0 {
            return;
        }

        queue.write_buffer(
            &self.time_buffer,
            0,
            bytemuck::cast_slice(&[StarfieldUniform {
                time,
                _pad0: 0.0,
                _pad1: 0.0,
                _pad2: 0.0,
            }]),
        );

        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, camera_bind_group, &[]);
        render_pass.set_bind_group(1, &self.time_bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.instance_buffer.slice(..));
        // 6 vertices per star (two-triangle quad), star_count instances.
        render_pass.draw(0..6, 0..self.star_count);
    }

    /// Generate a random starfield.
    pub fn generate_star_field(rng: &mut crate::util::Rng, count: u32) -> Vec<StarInstance> {
        let mut stars = Vec::with_capacity(count as usize);
        for _ in 0..count {
            stars.push(StarInstance {
                position: [
                    rng.range_f64(-50.0, 50.0) as f32,
                    rng.range_f64(-50.0, 50.0) as f32,
                ],
                brightness: rng.range_f64(0.1, 1.0) as f32,
                parallax_factor: rng.range_f64(0.0, 0.5) as f32,
                phase: rng.range_f64(0.0, std::f64::consts::TAU) as f32,
                _pad: [0.0; 3],
            });
        }
        stars
    }
}
