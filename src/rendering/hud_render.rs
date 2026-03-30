use wgpu::util::DeviceExt;
use super::text::BitmapFont;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct HudVertex {
    pub position: [f32; 2],
    pub uv: [f32; 2],
    pub color: [f32; 4],
}

#[derive(Clone, Debug)]
pub enum HudElement {
    Rect { x: f32, y: f32, w: f32, h: f32, color: [f32; 4] },
    Text { x: f32, y: f32, text: String, color: [f32; 4], scale: f32 },
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct HudUniform {
    screen_proj: [[f32; 4]; 4],
    tint_color: [f32; 4],
    use_texture: u32,
    _pad0: u32,
    _pad1: u32,
    _pad2: u32,
}

const MAX_HUD_VERTS: u32 = 16384;

pub struct HudPipeline {
    pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    // Two uniform buffers + bind groups: one for rects (use_texture=0), one for text (use_texture=1)
    rect_uniform_buffer: wgpu::Buffer,
    text_uniform_buffer: wgpu::Buffer,
    rect_bind_group: wgpu::BindGroup,
    text_bind_group: wgpu::BindGroup,
    font: BitmapFont,
    viewport_width: f32,
    viewport_height: f32,
}

impl HudPipeline {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
        viewport_width: f32,
        viewport_height: f32,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("hud_shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/hud.wgsl").into()),
        });

        let font = BitmapFont::new(device, queue);

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("hud_bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let screen_proj = orthographic_screen(viewport_width, viewport_height);

        let rect_uniform = HudUniform {
            screen_proj,
            tint_color: [1.0; 4],
            use_texture: 0,
            _pad0: 0, _pad1: 0, _pad2: 0,
        };
        let text_uniform = HudUniform {
            screen_proj,
            tint_color: [1.0; 4],
            use_texture: 1,
            _pad0: 0, _pad1: 0, _pad2: 0,
        };

        let rect_uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("hud_rect_uniform"),
            contents: bytemuck::cast_slice(&[rect_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let text_uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("hud_text_uniform"),
            contents: bytemuck::cast_slice(&[text_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("hud_sampler"),
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let rect_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("hud_rect_bg"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: rect_uniform_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&font.texture_view) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Sampler(&sampler) },
            ],
        });
        let text_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("hud_text_bg"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: text_uniform_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&font.texture_view) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Sampler(&sampler) },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("hud_pipeline_layout"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });

        let vertex_buffer_layout = wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<HudVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x2, offset: 0, shader_location: 0 },
                wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x2, offset: 8, shader_location: 1 },
                wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x4, offset: 16, shader_location: 2 },
            ],
        };

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("hud_pipeline"),
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
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("hud_vertex_buffer"),
            size: (std::mem::size_of::<HudVertex>() as u64) * MAX_HUD_VERTS as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            pipeline,
            vertex_buffer,
            rect_uniform_buffer,
            text_uniform_buffer,
            rect_bind_group,
            text_bind_group,
            font,
            viewport_width,
            viewport_height,
        }
    }

    pub fn resize(&mut self, _device: &wgpu::Device, width: f32, height: f32) {
        self.viewport_width = width;
        self.viewport_height = height;
    }

    /// Upload all HUD data to GPU. Call BEFORE beginning the render pass.
    /// Returns (rect_vertex_count, text_vertex_count).
    pub fn prepare(&self, queue: &wgpu::Queue, elements: &[HudElement]) -> (u32, u32) {
        if elements.is_empty() {
            return (0, 0);
        }

        let screen_proj = orthographic_screen(self.viewport_width, self.viewport_height);

        // Update both uniform buffers with the current projection
        queue.write_buffer(&self.rect_uniform_buffer, 0, bytemuck::cast_slice(&[HudUniform {
            screen_proj, tint_color: [1.0; 4], use_texture: 0,
            _pad0: 0, _pad1: 0, _pad2: 0,
        }]));
        queue.write_buffer(&self.text_uniform_buffer, 0, bytemuck::cast_slice(&[HudUniform {
            screen_proj, tint_color: [1.0; 4], use_texture: 1,
            _pad0: 0, _pad1: 0, _pad2: 0,
        }]));

        // Build vertex data: rects first, then text
        let mut all_verts: Vec<HudVertex> = Vec::new();
        let mut rect_count: u32 = 0;

        for el in elements {
            if let HudElement::Rect { x, y, w, h, color } = el {
                let (x, y, w, h) = (*x, *y, *w, *h);
                let c = *color;
                all_verts.push(HudVertex { position: [x, y], uv: [0.0, 0.0], color: c });
                all_verts.push(HudVertex { position: [x + w, y], uv: [0.0, 0.0], color: c });
                all_verts.push(HudVertex { position: [x + w, y + h], uv: [0.0, 0.0], color: c });
                all_verts.push(HudVertex { position: [x, y], uv: [0.0, 0.0], color: c });
                all_verts.push(HudVertex { position: [x + w, y + h], uv: [0.0, 0.0], color: c });
                all_verts.push(HudVertex { position: [x, y + h], uv: [0.0, 0.0], color: c });
                rect_count += 6;
            }
        }

        let mut text_count: u32 = 0;
        for el in elements {
            if let HudElement::Text { x, y, text, color, scale } = el {
                let verts = self.font.build_text_vertices(text, *x, *y, *scale, *color);
                text_count += verts.len() as u32;
                all_verts.extend_from_slice(&verts);
            }
        }

        let total = all_verts.len().min(MAX_HUD_VERTS as usize);
        if total > 0 {
            queue.write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&all_verts[..total]));
        }

        (rect_count.min(total as u32), text_count.min(total.saturating_sub(rect_count as usize) as u32))
    }

    /// Draw HUD elements during an active render pass.
    /// Call prepare() BEFORE beginning the render pass.
    pub fn render<'a>(
        &'a self,
        render_pass: &mut wgpu::RenderPass<'a>,
        rect_count: u32,
        text_count: u32,
    ) {
        if rect_count == 0 && text_count == 0 {
            return;
        }

        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));

        // Draw rects with rect bind group (use_texture = 0)
        if rect_count > 0 {
            render_pass.set_bind_group(0, &self.rect_bind_group, &[]);
            render_pass.draw(0..rect_count, 0..1);
        }

        // Draw text with text bind group (use_texture = 1)
        if text_count > 0 {
            render_pass.set_bind_group(0, &self.text_bind_group, &[]);
            render_pass.draw(rect_count..rect_count + text_count, 0..1);
        }
    }
}

fn orthographic_screen(width: f32, height: f32) -> [[f32; 4]; 4] {
    [
        [2.0 / width, 0.0, 0.0, 0.0],
        [0.0, -2.0 / height, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [-1.0, 1.0, 0.0, 1.0],
    ]
}
