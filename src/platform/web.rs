#![cfg(target_arch = "wasm32")]

use std::sync::Arc;

use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowId};

use wasm_bindgen::JsCast;

use crate::audio::{AudioBackend, create_audio_backend};
use crate::camera::Camera;
use crate::game::{Game, GameState, NameEntryState};
use crate::input::touch::TouchInput;
use crate::input::keyboard_mouse::KeyboardMouseInput;
use crate::input::InputProvider;
use crate::persistence::SaveData;
use crate::persistence::web_save::WebSave;
use crate::rendering::Renderer;
use crate::rendering::gpu::init_gpu;

enum GpuState {
    NotStarted,
    Pending,
    Ready,
}

struct App {
    window: Option<Arc<Window>>,
    renderer: Option<Renderer>,
    game: Option<Game>,
    audio: Box<dyn AudioBackend>,
    keyboard_input: KeyboardMouseInput,
    touch_input: Option<TouchInput>,
    camera: Camera,
    last_frame_time: Option<f64>,
    total_time: f32,
    gpu_state: GpuState,
    save_backend: WebSave,
}

impl App {
    fn new() -> Self {
        Self {
            window: None,
            renderer: None,
            game: None,
            audio: create_audio_backend(),
            keyboard_input: KeyboardMouseInput::new(),
            touch_input: None,
            camera: Camera::new(800.0, 600.0),
            last_frame_time: None,
            total_time: 0.0,
            gpu_state: GpuState::NotStarted,
            save_backend: WebSave::new(),
        }
    }

    fn performance_now() -> f64 {
        web_sys::window()
            .and_then(|w| w.performance())
            .map(|p| p.now() / 1000.0)
            .unwrap_or(0.0)
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let canvas = web_sys::window()
            .and_then(|w| w.document())
            .and_then(|doc| doc.get_element_by_id("game-canvas"))
            .and_then(|el| el.dyn_into::<web_sys::HtmlCanvasElement>().ok())
            .expect("Could not find canvas #game-canvas");

        // Use LogicalSize so winit handles DPR scaling for us.
        // winit will set the canvas buffer to physical pixels internally.
        let win_js = web_sys::window().unwrap();
        let css_w = win_js.inner_width().ok().and_then(|v| v.as_f64()).unwrap_or(1280.0);
        let css_h = win_js.inner_height().ok().and_then(|v| v.as_f64()).unwrap_or(720.0);

        use winit::platform::web::WindowAttributesExtWebSys;
        let window_attrs = Window::default_attributes()
            .with_title("Gravity Well Arena")
            .with_canvas(Some(canvas))
            .with_prevent_default(true)
            .with_inner_size(winit::dpi::LogicalSize::new(css_w, css_h));

        let window = Arc::new(
            event_loop.create_window(window_attrs).expect("Failed to create window"),
        );

        // winit gives us physical size — use that for everything
        let size = window.inner_size();
        let pw = size.width.max(1);
        let ph = size.height.max(1);

        // Set initial canvas CSS size to viewport
        if let Some(doc) = win_js.document() {
            if let Some(el) = doc.get_element_by_id("game-canvas") {
                let _ = el.dyn_ref::<web_sys::HtmlCanvasElement>().map(|c| {
                    let _ = c.style().set_property("width", &format!("{}px", css_w as u32));
                    let _ = c.style().set_property("height", &format!("{}px", css_h as u32));
                });
            }
        }

        self.camera = Camera::new(pw as f32, ph as f32);
        self.touch_input = Some(TouchInput::new(pw as f32, ph as f32));
        self.window = Some(window.clone());
        self.gpu_state = GpuState::Pending;

        let win = window.clone();
        let w = pw;
        let h = ph;

        let renderer_slot: *mut Option<Renderer> = &mut self.renderer;
        let game_slot: *mut Option<Game> = &mut self.game;
        let gpu_state_slot: *mut GpuState = &mut self.gpu_state;

        wasm_bindgen_futures::spawn_local(async move {
            let (device, queue, surface, config) = init_gpu(win).await;
            let renderer = Renderer::new(device, queue, surface, config);

            let mut game = Game::new(w as f32, h as f32);
            game.dpi_scale = 1.0; // winit physical pixels = render pixels, no extra scaling
            let web_save = WebSave::new();
            game.load_save(&web_save);
            if game.display_name.is_empty() {
                game.state = GameState::NameEntry;
                game.name_entry = Some(NameEntryState::new());
            } else {
                let start_level = game.progression.highest_level + 1;
                game.start_level(start_level.max(1));
            }

            unsafe {
                *renderer_slot = Some(renderer);
                *game_slot = Some(game);
                *gpu_state_slot = GpuState::Ready;
            }
        });
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
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
                if let Some(touch) = self.touch_input.as_mut() {
                    *touch = TouchInput::new(width as f32, height as f32);
                }
            }

            WindowEvent::RedrawRequested => {
                if self.renderer.is_none() || self.game.is_none() {
                    if let Some(window) = self.window.as_ref() {
                        window.request_redraw();
                    }
                    return;
                }

                // Detect browser viewport resize by comparing current window
                // inner_size with the renderer's surface dimensions.
                if let (Some(window), Some(renderer)) =
                    (self.window.as_ref(), self.renderer.as_ref())
                {
                    // Ask winit for the current logical size from the browser
                    if let Some(win_js) = web_sys::window() {
                        let css_w = win_js.inner_width().ok()
                            .and_then(|v| v.as_f64()).unwrap_or(0.0);
                        let css_h = win_js.inner_height().ok()
                            .and_then(|v| v.as_f64()).unwrap_or(0.0);
                        if css_w > 0.0 && css_h > 0.0 {
                            let logical = winit::dpi::LogicalSize::new(css_w, css_h);
                            let physical: winit::dpi::PhysicalSize<u32> =
                                logical.to_physical(window.scale_factor());
                            let cur_w = renderer.surface_config.width;
                            let cur_h = renderer.surface_config.height;
                            if physical.width != cur_w || physical.height != cur_h {
                                let w = physical.width.max(1);
                                let h = physical.height.max(1);

                                // Update canvas CSS size to match viewport exactly
                                if let Some(doc) = win_js.document() {
                                    if let Some(el) = doc.get_element_by_id("game-canvas") {
                                        if let Ok(canvas) = el.dyn_into::<web_sys::HtmlCanvasElement>() {
                                            // Set buffer to physical pixels
                                            canvas.set_width(w);
                                            canvas.set_height(h);
                                            // Set CSS display to CSS pixels (viewport size)
                                            let _ = canvas.style().set_property(
                                                "width", &format!("{}px", css_w as u32));
                                            let _ = canvas.style().set_property(
                                                "height", &format!("{}px", css_h as u32));
                                        }
                                    }
                                }

                                if let Some(renderer) = self.renderer.as_mut() {
                                    renderer.resize(w, h);
                                }
                                self.camera.resize(w as f32, h as f32);
                                if let Some(game) = self.game.as_mut() {
                                    game.camera.resize(w as f32, h as f32);
                                }
                                if let Some(touch) = self.touch_input.as_mut() {
                                    *touch = TouchInput::new(w as f32, h as f32);
                                }
                            }
                        }
                    }
                }

                let now = Self::performance_now();
                let dt = match self.last_frame_time {
                    Some(last) => (now - last).min(0.1),
                    None => 1.0 / 60.0,
                };
                self.last_frame_time = Some(now);
                self.total_time += dt as f32;

                let mut actions = self.keyboard_input.poll(&self.camera);
                if let Some(touch) = self.touch_input.as_mut() {
                    actions.extend(touch.poll(&self.camera));
                }

                if let Some(game) = self.game.as_mut() {
                    game.update(dt, &actions, self.audio.as_mut());

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
                // All events (keyboard, mouse, touch) go through winit's coordinate system
                // which is physical pixels — matching our render coordinate system.
                self.keyboard_input.handle_window_event(&other);
                if let Some(touch) = self.touch_input.as_mut() {
                    touch.handle_window_event(&other);
                }
            }
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(window) = self.window.as_ref() {
            window.request_redraw();
        }
    }
}

pub fn run_web() {
    console_error_panic_hook::set_once();
    let _ = console_log::init_with_level(log::Level::Info);

    let event_loop = EventLoop::new().expect("Failed to create event loop");
    let mut app = App::new();
    event_loop.run_app(&mut app).expect("Event loop error");
}
