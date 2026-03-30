use std::sync::Arc;

/// Camera data sent to the GPU as a uniform buffer.
#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    pub view_proj: [[f32; 4]; 4],
    pub camera_pos: [f32; 2],
    pub zoom: f32,
    pub _pad0: f32,
    pub viewport_size: [f32; 2],
    pub _pad1: [f32; 2],
}

impl CameraUniform {
    pub fn from_camera(camera: &crate::camera::Camera) -> Self {
        Self {
            view_proj: camera.view_projection_matrix(),
            camera_pos: [camera.position.x as f32, camera.position.y as f32],
            zoom: camera.zoom as f32,
            _pad0: 0.0,
            viewport_size: [camera.viewport_width, camera.viewport_height],
            _pad1: [0.0; 2],
        }
    }

    pub fn identity() -> Self {
        Self {
            view_proj: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
            camera_pos: [0.0; 2],
            zoom: 1.0,
            _pad0: 0.0,
            viewport_size: [800.0, 600.0],
            _pad1: [0.0; 2],
        }
    }
}

/// Initialize the wgpu device, queue, surface, and surface configuration.
pub async fn init_gpu(
    window: Arc<winit::window::Window>,
) -> (
    wgpu::Device,
    wgpu::Queue,
    wgpu::Surface<'static>,
    wgpu::SurfaceConfiguration,
) {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::all(),
        ..Default::default()
    });

    let surface = match instance.create_surface(window.clone()) {
        Ok(s) => s,
        Err(_e) => {
            #[cfg(target_arch = "wasm32")]
            {
                wasm_bindgen::throw_str("WebGPU not available. Enable it in your browser settings.");
            }
            #[cfg(not(target_arch = "wasm32"))]
            {
                panic!("Failed to create surface: {_e}");
            }
        }
    };

    // Try high-performance first, fall back to low power, then fallback adapter
    let adapter = {
        let hp = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await;

        match hp {
            Some(a) => a,
            None => {
                let lp = instance
                    .request_adapter(&wgpu::RequestAdapterOptions {
                        power_preference: wgpu::PowerPreference::LowPower,
                        compatible_surface: Some(&surface),
                        force_fallback_adapter: false,
                    })
                    .await;
                match lp {
                    Some(a) => a,
                    None => {
                        // On web: throw a JS error that bootstrap.js can catch.
                        // On native: panic (no recovery possible).
                        #[cfg(target_arch = "wasm32")]
                        {
                            wasm_bindgen::throw_str("No GPU adapter available. Enable WebGPU in your browser settings (chrome://flags/#enable-unsafe-webgpu)");
                        }
                        #[cfg(not(target_arch = "wasm32"))]
                        {
                            panic!("No GPU adapter available");
                        }
                    }
                }
            }
        }
    };

    let (device, queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                label: Some("gravity_device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                ..Default::default()
            },
            None,
        )
        .await
        .expect("Failed to create device");

    let size = window.inner_size();
    let caps = surface.get_capabilities(&adapter);

    // Prefer Bgra8UnormSrgb, fall back to the first available format.
    let format = caps
        .formats
        .iter()
        .find(|f| **f == wgpu::TextureFormat::Bgra8UnormSrgb)
        .copied()
        .unwrap_or(caps.formats[0]);

    let present_mode = if caps.present_modes.contains(&wgpu::PresentMode::Mailbox) {
        wgpu::PresentMode::Mailbox
    } else {
        wgpu::PresentMode::Fifo
    };

    let config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format,
        width: size.width.max(1),
        height: size.height.max(1),
        present_mode,
        alpha_mode: caps.alpha_modes[0],
        view_formats: vec![],
        desired_maximum_frame_latency: 2,
    };
    surface.configure(&device, &config);

    (device, queue, surface, config)
}
