/// Instance data for a ship or projectile sprite.
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ShipInstance {
    pub position: [f32; 2],
    pub rotation: f32,
    pub scale: f32,
    pub color: [f32; 4],
    pub shield_alpha: f32,
    pub thrust_type: u32,
    pub thrust_magnitude: f32,
    pub turret_angle: f32,
}

/// Projectile instances reuse ShipInstance with different visual parameters.
pub type ProjectileInstance = ShipInstance;

const MAX_INSTANCES: u32 = 512;
/// 9 vertices per instance: 3 hull + 3 thrust + 3 turret.
const VERTS_PER_INSTANCE: u32 = 9;

pub struct SpritePipeline {
    pipeline: wgpu::RenderPipeline,
    instance_buffer: wgpu::Buffer,
    max_instances: u32,
}

impl SpritePipeline {
    pub fn new(
        device: &wgpu::Device,
        camera_bind_group_layout: &wgpu::BindGroupLayout,
        format: wgpu::TextureFormat,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("sprite_shader"),
            source: wgpu::ShaderSource::Wgsl(
                include_str!("../../shaders/sprite.wgsl").into(),
            ),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("sprite_pipeline_layout"),
            bind_group_layouts: &[camera_bind_group_layout],
            push_constant_ranges: &[],
        });

        let instance_buffer_layout = wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<ShipInstance>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                // position
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x2,
                    offset: 0,
                    shader_location: 0,
                },
                // rotation
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32,
                    offset: 8,
                    shader_location: 1,
                },
                // scale
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32,
                    offset: 12,
                    shader_location: 2,
                },
                // color (vec4)
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x4,
                    offset: 16,
                    shader_location: 3,
                },
                // shield_alpha
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32,
                    offset: 32,
                    shader_location: 4,
                },
                // thrust_type
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Uint32,
                    offset: 36,
                    shader_location: 5,
                },
                // thrust_magnitude
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32,
                    offset: 40,
                    shader_location: 6,
                },
                // turret_angle
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32,
                    offset: 44,
                    shader_location: 7,
                },
            ],
        };

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("sprite_pipeline"),
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
            label: Some("sprite_instance_buffer"),
            size: (std::mem::size_of::<ShipInstance>() as u64) * MAX_INSTANCES as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            pipeline,
            instance_buffer,
            max_instances: MAX_INSTANCES,
        }
    }

    /// Update instance data on the GPU. Returns the number of instances written.
    pub fn update_instances(
        &self,
        queue: &wgpu::Queue,
        instances: &[ShipInstance],
    ) -> u32 {
        let count = instances.len().min(self.max_instances as usize);
        if count > 0 {
            queue.write_buffer(
                &self.instance_buffer,
                0,
                bytemuck::cast_slice(&instances[..count]),
            );
        }
        count as u32
    }

    pub fn render<'a>(
        &'a self,
        render_pass: &mut wgpu::RenderPass<'a>,
        camera_bind_group: &'a wgpu::BindGroup,
        instance_count: u32,
    ) {
        if instance_count == 0 {
            return;
        }
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, camera_bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.instance_buffer.slice(..));
        render_pass.draw(0..VERTS_PER_INSTANCE, 0..instance_count);
    }
}
