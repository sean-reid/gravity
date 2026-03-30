/// Per-black-hole data sent to the GPU.
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct BlackHoleData {
    pub position: [f32; 2],
    pub radius: f32,
    pub time: f32,
}

const MAX_BLACK_HOLES: u32 = 8;

pub struct BlackHolePipeline {
    pipeline: wgpu::RenderPipeline,
    bh_buffer: wgpu::Buffer,
    bh_bind_group_layout: wgpu::BindGroupLayout,
    bh_bind_groups: Vec<wgpu::BindGroup>,
    bh_count: u32,
}

impl BlackHolePipeline {
    pub fn new(
        device: &wgpu::Device,
        camera_bind_group_layout: &wgpu::BindGroupLayout,
        format: wgpu::TextureFormat,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("blackhole_shader"),
            source: wgpu::ShaderSource::Wgsl(
                include_str!("../shaders/blackhole.wgsl").into(),
            ),
        });

        let bh_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("blackhole_bgl"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: true,
                        min_binding_size: wgpu::BufferSize::new(
                            std::mem::size_of::<BlackHoleData>() as u64,
                        ),
                    },
                    count: None,
                }],
            });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("blackhole_pipeline_layout"),
            bind_group_layouts: &[Some(camera_bind_group_layout), Some(&bh_bind_group_layout)],
            immediate_size: 0,
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("blackhole_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[],
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
            multiview_mask: None,
            cache: None,
        });

        // Allocate a buffer large enough for all black holes, aligned to 256 bytes
        // (the minimum uniform buffer offset alignment).
        let aligned_size = align_to(std::mem::size_of::<BlackHoleData>(), 256);
        let bh_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("blackhole_uniform_buffer"),
            size: (aligned_size * MAX_BLACK_HOLES as usize) as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bh_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("blackhole_bg"),
            layout: &bh_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &bh_buffer,
                    offset: 0,
                    size: wgpu::BufferSize::new(std::mem::size_of::<BlackHoleData>() as u64),
                }),
            }],
        });

        Self {
            pipeline,
            bh_buffer,
            bh_bind_group_layout,
            bh_bind_groups: vec![bh_bind_group],
            bh_count: 0,
        }
    }

    /// Upload black hole data. Each BH gets a separate dynamic offset draw.
    pub fn update_black_holes(&mut self, queue: &wgpu::Queue, holes: &[BlackHoleData]) {
        let count = holes.len().min(MAX_BLACK_HOLES as usize);
        self.bh_count = count as u32;

        let aligned_size = align_to(std::mem::size_of::<BlackHoleData>(), 256);

        for i in 0..count {
            let mut padded = vec![0u8; aligned_size];
            let bytes = bytemuck::bytes_of(&holes[i]);
            padded[..bytes.len()].copy_from_slice(bytes);
            queue.write_buffer(
                &self.bh_buffer,
                (i * aligned_size) as u64,
                &padded,
            );
        }
    }

    pub fn render<'a>(
        &'a self,
        render_pass: &mut wgpu::RenderPass<'a>,
        camera_bind_group: &'a wgpu::BindGroup,
    ) {
        if self.bh_count == 0 {
            return;
        }
        let aligned_size = align_to(std::mem::size_of::<BlackHoleData>(), 256);

        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, camera_bind_group, &[]);

        for i in 0..self.bh_count {
            let offset = (i as usize * aligned_size) as u32;
            render_pass.set_bind_group(1, &self.bh_bind_groups[0], &[offset]);
            render_pass.draw(0..6, 0..1);
        }
    }
}

/// Align `size` up to `alignment` (must be power of two).
fn align_to(size: usize, alignment: usize) -> usize {
    (size + alignment - 1) & !(alignment - 1)
}
