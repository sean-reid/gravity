#![cfg(not(target_arch = "wasm32"))]

use std::sync::Arc;

use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowId};

use crate::audio::{AudioBackend, create_audio_backend};
use crate::camera::Camera;
use crate::game::{Game, GameState, NameEntryState};
use crate::input::KeyboardMouseInput;
use crate::input::InputProvider;
use crate::persistence::SaveData;
use crate::persistence::native_save::NativeSave;
use crate::rendering::Renderer;
use crate::rendering::gpu::init_gpu;

struct App {
    window: Option<Arc<Window>>,
    renderer: Option<Renderer>,
    game: Option<Game>,
    audio: Box<dyn AudioBackend>,
    input: KeyboardMouseInput,
    camera: Camera,
    save_backend: NativeSave,
    last_frame_time: std::time::Instant,
    total_time: f32,
}

impl App {
    fn new() -> Self {
        Self {
            window: None,
            renderer: None,
            game: None,
            audio: create_audio_backend(),
            input: KeyboardMouseInput::new(),
            camera: Camera::new(1280.0, 720.0),
            save_backend: NativeSave::new(),
            last_frame_time: std::time::Instant::now(),
            total_time: 0.0,
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let window_attrs = Window::default_attributes()
            .with_title("Gravity Well Arena")
            .with_inner_size(winit::dpi::LogicalSize::new(1280, 720));

        let window = Arc::new(
            event_loop
                .create_window(window_attrs)
                .expect("Failed to create window"),
        );

        let size = window.inner_size();
        let width = size.width.max(1);
        let height = size.height.max(1);

        let (device, queue, surface, config) =
            pollster::block_on(init_gpu(window.clone()));

        let renderer = Renderer::new(device, queue, surface, config);

        self.camera = Camera::new(width as f32, height as f32);

        let mut game = Game::new(width as f32, height as f32);
        game.dpi_scale = window.scale_factor() as f32;

        // Load saved progress
        game.load_save(&self.save_backend);

        if game.display_name.is_empty() {
            // No saved name -- show name entry screen
            game.state = GameState::NameEntry;
            game.name_entry = Some(NameEntryState::new());
        } else {
            // Start from the next unbeaten level, or level 1 if no save
            let start_level = game.progression.highest_level + 1;
            game.start_level(start_level.max(1));
        }

        self.window = Some(window);
        self.renderer = Some(renderer);
        self.game = Some(game);
        self.last_frame_time = std::time::Instant::now();
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                // Save on exit
                if let Some(game) = self.game.as_ref() {
                    game.save_game(&self.save_backend);
                }
                event_loop.exit();
            }

            WindowEvent::Resized(size) => {
                let width = size.width.max(1);
                let height = size.height.max(1);

                if let Some(renderer) = self.renderer.as_mut() {
                    renderer.resize(width, height);
                }

                self.camera.resize(width as f32, height as f32);

                if let Some(game) = self.game.as_mut() {
                    game.camera.resize(width as f32, height as f32);
                }

                // Update DPI scale in case scale factor changed
                if let Some(window) = self.window.as_ref() {
                    if let Some(game) = self.game.as_mut() {
                        game.dpi_scale = window.scale_factor() as f32;
                    }
                }
            }

            WindowEvent::RedrawRequested => {
                let now = std::time::Instant::now();
                let dt = now.duration_since(self.last_frame_time).as_secs_f64();
                self.last_frame_time = now;

                let dt = dt.min(0.1);
                self.total_time += dt as f32;

                let actions = self.input.poll(&self.camera);

                if let Some(game) = self.game.as_mut() {
                    game.update(dt, &actions, self.audio.as_mut());

                    // Check if game wants to save
                    if game.needs_save {
                        game.save_game(&self.save_backend);
                        game.needs_save = false;
                    }

                    self.camera.position = game.camera.position;
                    self.camera.zoom = game.camera.zoom;
                    self.camera.viewport_width = game.camera.viewport_width;
                    self.camera.viewport_height = game.camera.viewport_height;
                }

                if let (Some(game), Some(renderer)) =
                    (self.game.as_ref(), self.renderer.as_mut())
                {
                    let scene = game.build_render_scene(self.total_time);
                    renderer.render(&scene);
                }

                if let Some(window) = self.window.as_ref() {
                    window.request_redraw();
                }
            }

            other => {
                self.input.handle_window_event(&other);
            }
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(window) = self.window.as_ref() {
            window.request_redraw();
        }
    }
}

pub fn run_native() {
    let event_loop = EventLoop::new().expect("Failed to create event loop");
    let mut app = App::new();
    event_loop.run_app(&mut app).expect("Event loop error");
}
