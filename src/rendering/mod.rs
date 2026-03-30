pub mod gpu;
pub mod pipelines;
pub mod black_hole;
pub mod hud_render;
pub mod text;
pub mod ships;
pub mod projectiles;

pub use gpu::CameraUniform;
pub use pipelines::{
    StarInstance, ShipInstance, ProjectileInstance,
    TrailData, TrailVertex,
    ParticleInstance, BeamSegment,
    PostprocessParams,
};
pub use black_hole::BlackHoleData;
pub use hud_render::HudElement;

use pipelines::{
    StarfieldPipeline, SpritePipeline, TrailPipeline,
    ParticlePipeline, BeamPipeline, PostprocessPipeline,
};
use black_hole::BlackHolePipeline;
use hud_render::HudPipeline;

use wgpu::util::DeviceExt;

/// A snapshot of everything to render in one frame.
pub struct RenderScene {
    pub camera: CameraUniform,
    pub stars: Vec<StarInstance>,
    pub black_holes: Vec<BlackHoleData>,
    pub trails: Vec<TrailData>,
    pub ship_instances: Vec<ShipInstance>,
    pub projectile_instances: Vec<ShipInstance>,
    pub beam_segments: Vec<BeamSegment>,
    pub particles: Vec<ParticleInstance>,
    pub hud_elements: Vec<HudElement>,
    pub time: f32,
    pub depth_factor: f32,
}

/// Main renderer that owns the wgpu device/queue/surface and all pipelines.
pub struct Renderer {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub surface: wgpu::Surface<'static>,
    pub surface_config: wgpu::SurfaceConfiguration,
    // Pipeline objects
    starfield_pipeline: StarfieldPipeline,
    black_hole_pipeline: BlackHolePipeline,
    trail_pipeline: TrailPipeline,
    sprite_pipeline: SpritePipeline,
    beam_pipeline: BeamPipeline,
    particle_pipeline: ParticlePipeline,
    postprocess_pipeline: PostprocessPipeline,
    hud_pipeline: HudPipeline,
    // Shared resources
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    camera_bind_group_layout: wgpu::BindGroupLayout,
    scene_texture: wgpu::Texture,
    scene_texture_view: wgpu::TextureView,
    postprocess_bind_group: wgpu::BindGroup,
    depth_texture: Option<wgpu::TextureView>,
}

impl Renderer {
    /// Create a new renderer with all pipelines initialized.
    pub fn new(
        device: wgpu::Device,
        queue: wgpu::Queue,
        surface: wgpu::Surface<'static>,
        config: wgpu::SurfaceConfiguration,
    ) -> Self {
        let camera_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("camera_bgl"),
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

        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("camera_uniform_buffer"),
            contents: bytemuck::cast_slice(&[CameraUniform::identity()]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("camera_bg"),
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
        });

        // Scene framebuffer for offscreen rendering (post-processing input).
        let scene_format = config.format;
        let (scene_texture, scene_texture_view) =
            create_scene_texture(&device, config.width, config.height, scene_format);

        // Pipelines
        let starfield_pipeline =
            StarfieldPipeline::new(&device, &camera_bind_group_layout, scene_format);
        let black_hole_pipeline =
            BlackHolePipeline::new(&device, &camera_bind_group_layout, scene_format);
        let trail_pipeline =
            TrailPipeline::new(&device, &camera_bind_group_layout, scene_format);
        let sprite_pipeline =
            SpritePipeline::new(&device, &camera_bind_group_layout, scene_format);
        let beam_pipeline =
            BeamPipeline::new(&device, &camera_bind_group_layout, scene_format);
        let particle_pipeline =
            ParticlePipeline::new(&device, &camera_bind_group_layout, scene_format);
        let postprocess_pipeline = PostprocessPipeline::new(&device, config.format);
        let hud_pipeline = HudPipeline::new(
            &device,
            &queue,
            config.format,
            config.width as f32,
            config.height as f32,
        );

        let postprocess_bind_group =
            postprocess_pipeline.create_bind_group(&device, &scene_texture_view);

        Self {
            device,
            queue,
            surface,
            surface_config: config,
            starfield_pipeline,
            black_hole_pipeline,
            trail_pipeline,
            sprite_pipeline,
            beam_pipeline,
            particle_pipeline,
            postprocess_pipeline,
            hud_pipeline,
            camera_buffer,
            camera_bind_group,
            camera_bind_group_layout,
            scene_texture,
            scene_texture_view,
            postprocess_bind_group,
            depth_texture: None,
        }
    }

    /// Handle window resize.
    pub fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        self.surface_config.width = width;
        self.surface_config.height = height;
        self.surface.configure(&self.device, &self.surface_config);

        let scene_format = self.surface_config.format;
        let (tex, view) = create_scene_texture(&self.device, width, height, scene_format);
        self.scene_texture = tex;
        self.scene_texture_view = view;

        self.postprocess_bind_group = self
            .postprocess_pipeline
            .create_bind_group(&self.device, &self.scene_texture_view);

        self.hud_pipeline
            .resize(&self.device, width as f32, height as f32);
    }

    /// Execute all render passes for the given scene.
    pub fn render(&mut self, scene: &RenderScene) {
        // Update camera uniform.
        self.queue.write_buffer(
            &self.camera_buffer,
            0,
            bytemuck::cast_slice(&[scene.camera]),
        );

        // Update pipeline data.
        self.starfield_pipeline
            .update_stars(&self.queue, &scene.stars);
        self.black_hole_pipeline
            .update_black_holes(&self.queue, &scene.black_holes);
        self.trail_pipeline.update_trails(&self.queue, &scene.trails);

        // Combine ship + projectile instances for sprite pass.
        let mut all_sprites = Vec::with_capacity(
            scene.ship_instances.len() + scene.projectile_instances.len(),
        );
        all_sprites.extend_from_slice(&scene.ship_instances);
        all_sprites.extend_from_slice(&scene.projectile_instances);
        let sprite_count = self.sprite_pipeline.update_instances(&self.queue, &all_sprites);

        self.beam_pipeline
            .update_beams(&self.queue, &scene.beam_segments);
        self.particle_pipeline
            .update_particles(&self.queue, &scene.particles);

        // Update postprocess params.
        let mut pp_params = PostprocessParams {
            viewport_size: [
                self.surface_config.width as f32,
                self.surface_config.height as f32,
            ],
            depth_factor: scene.depth_factor,
            num_black_holes: scene.black_holes.len().min(8) as u32,
            black_holes: [[0.0; 4]; 8],
        };
        // Convert black hole world positions to UV-space for postprocessing.
        for (i, bh) in scene.black_holes.iter().take(8).enumerate() {
            // Approximate screen UV from world position using camera data.
            let vp = &scene.camera;
            let ndc_x = vp.view_proj[0][0] * bh.position[0]
                + vp.view_proj[3][0];
            let ndc_y = vp.view_proj[1][1] * bh.position[1]
                + vp.view_proj[3][1];
            let uv_x = (ndc_x + 1.0) * 0.5;
            let uv_y = (-ndc_y + 1.0) * 0.5;
            // Radius in UV space: approximate using x scale.
            let radius_uv = bh.radius * vp.view_proj[0][0].abs() * 0.5;
            pp_params.black_holes[i] = [uv_x, uv_y, radius_uv, 0.0];
        }
        self.postprocess_pipeline
            .update_params(&self.queue, &pp_params);

        // Acquire surface texture.
        let output = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(t) | wgpu::CurrentSurfaceTexture::Suboptimal(t) => t,
            wgpu::CurrentSurfaceTexture::Lost | wgpu::CurrentSurfaceTexture::Outdated => {
                self.surface
                    .configure(&self.device, &self.surface_config);
                return;
            }
            other => {
                // Occluded = window minimized/hidden, harmless — skip this frame
                log::trace!("Surface unavailable: {:?}", other);
                return;
            }
        };
        let output_view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder =
            self.device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("render_encoder"),
                });

        // ---- Pass 1-6: Render to offscreen scene texture ----
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("scene_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.scene_texture_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.0,
                            g: 0.0,
                            b: 0.02,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                ..Default::default()
            });

            // 1. Starfield
            self.starfield_pipeline
                .render(&mut pass, &self.camera_bind_group, scene.time, &self.queue);

            // 2. Black holes
            self.black_hole_pipeline
                .render(&mut pass, &self.camera_bind_group);

            // 3. Trails
            self.trail_pipeline
                .render(&mut pass, &self.camera_bind_group);

            // 4. Sprites (ships + projectiles)
            self.sprite_pipeline
                .render(&mut pass, &self.camera_bind_group, sprite_count);

            // 5. Beams
            self.beam_pipeline
                .render(&mut pass, &self.camera_bind_group);

            // 6. Particles
            self.particle_pipeline
                .render(&mut pass, &self.camera_bind_group);
        }

        // ---- Pass 7: Post-processing to surface ----
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("postprocess_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &output_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                ..Default::default()
            });

            self.postprocess_pipeline
                .render(&mut pass, &self.postprocess_bind_group);
        }

        // ---- Pass 8: HUD overlay on surface ----
        // Prepare HUD data BEFORE beginning the render pass (upload vertices/uniforms)
        let (hud_rect_count, hud_text_count) =
            self.hud_pipeline.prepare(&self.queue, &scene.hud_elements);
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("hud_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &output_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                ..Default::default()
            });

            self.hud_pipeline
                .render(&mut pass, &self.queue, hud_rect_count, hud_text_count);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();
    }
}

/// Create the offscreen scene texture and its view.
fn create_scene_texture(
    device: &wgpu::Device,
    width: u32,
    height: u32,
    format: wgpu::TextureFormat,
) -> (wgpu::Texture, wgpu::TextureView) {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("scene_texture"),
        size: wgpu::Extent3d {
            width: width.max(1),
            height: height.max(1),
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    (texture, view)
}
