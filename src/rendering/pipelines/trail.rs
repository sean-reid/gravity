/// Per-vertex data for trail lines.
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct TrailVertex {
    pub position: [f32; 2],
    pub alpha: f32,
    pub color: [f32; 3],
}

/// A single trail (one entity's trail).
pub struct TrailData {
    pub vertices: Vec<TrailVertex>,
}

/// Maximum total trail vertices across all entities.
const MAX_TRAIL_VERTS: u32 = 16384;

pub struct TrailPipeline {
    pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    /// (offset, count) ranges for each trail strip in the vertex buffer.
    trail_ranges: Vec<(u32, u32)>,
}

impl TrailPipeline {
    pub fn new(
        device: &wgpu::Device,
        camera_bind_group_layout: &wgpu::BindGroupLayout,
        format: wgpu::TextureFormat,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("trail_shader"),
            source: wgpu::ShaderSource::Wgsl(
                include_str!("../../shaders/trail.wgsl").into(),
            ),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("trail_pipeline_layout"),
            bind_group_layouts: &[Some(camera_bind_group_layout)],
            immediate_size: 0,
        });

        let vertex_buffer_layout = wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<TrailVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                // position
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x2,
                    offset: 0,
                    shader_location: 0,
                },
                // alpha
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32,
                    offset: 8,
                    shader_location: 1,
                },
                // color (vec3)
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x3,
                    offset: 12,
                    shader_location: 2,
                },
            ],
        };

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("trail_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[vertex_buffer_layout],
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
                topology: wgpu::PrimitiveTopology::LineStrip,
                strip_index_format: None,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("trail_vertex_buffer"),
            size: (std::mem::size_of::<TrailVertex>() as u64) * MAX_TRAIL_VERTS as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            pipeline,
            vertex_buffer,
            trail_ranges: Vec::new(),
        }
    }

    /// Upload all trails into a single vertex buffer.
    /// Each trail is a separate line strip draw call.
    pub fn update_trails(&mut self, queue: &wgpu::Queue, trails: &[TrailData]) {
        self.trail_ranges.clear();
        let mut all_verts: Vec<TrailVertex> = Vec::new();

        for trail in trails {
            if trail.vertices.len() < 2 {
                continue;
            }
            let offset = all_verts.len() as u32;
            let count = trail.vertices.len().min(
                (MAX_TRAIL_VERTS as usize).saturating_sub(all_verts.len()),
            ) as u32;
            if count < 2 {
                break;
            }
            all_verts.extend_from_slice(&trail.vertices[..count as usize]);
            self.trail_ranges.push((offset, count));

            if all_verts.len() >= MAX_TRAIL_VERTS as usize {
                break;
            }
        }

        if !all_verts.is_empty() {
            queue.write_buffer(
                &self.vertex_buffer,
                0,
                bytemuck::cast_slice(&all_verts),
            );
        }
    }

    pub fn render<'a>(
        &'a self,
        render_pass: &mut wgpu::RenderPass<'a>,
        camera_bind_group: &'a wgpu::BindGroup,
    ) {
        if self.trail_ranges.is_empty() {
            return;
        }
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, camera_bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));

        for &(offset, count) in &self.trail_ranges {
            render_pass.draw(offset..(offset + count), 0..1);
        }
    }
}
