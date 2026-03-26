use crate::util::Vec2;

pub struct Camera {
    pub position: Vec2,
    pub zoom: f64,
    pub viewport_width: f32,
    pub viewport_height: f32,
    pub target_position: Vec2,
    pub target_zoom: f64,
    min_zoom: f64,
    max_zoom: f64,
}

impl Camera {
    /// Create a new camera with the given viewport dimensions.
    /// Default zoom is 10.0 (shows ~10 Schwarzschild radii).
    pub fn new(viewport_width: f32, viewport_height: f32) -> Self {
        let default_zoom = 10.0;
        Self {
            position: Vec2::ZERO,
            zoom: default_zoom,
            viewport_width,
            viewport_height,
            target_position: Vec2::ZERO,
            target_zoom: default_zoom,
            min_zoom: 3.0,
            max_zoom: 40.0,
        }
    }

    /// Update the camera each frame. Smoothly lerp position and zoom toward targets.
    ///
    /// The target position is the player position offset 15% of the half-width toward
    /// the nearest black hole, keeping the threat visible on screen.
    pub fn update(&mut self, player_pos: Vec2, nearest_bh_pos: Vec2, dt: f64) {
        // Compute target position: player + 15% of half-width toward the black hole.
        let half_width = self.zoom; // zoom = world units in half-width
        let to_bh = nearest_bh_pos - player_pos;
        let to_bh_len = to_bh.length();
        let offset = if to_bh_len > 1e-6 {
            to_bh.normalized() * (half_width * 0.15)
        } else {
            Vec2::ZERO
        };
        self.target_position = player_pos + offset;

        // Smooth lerp with frame-rate independent factor.
        // factor = 1 - (1 - base)^(dt * 60), approximated for small dt.
        let lerp_base: f64 = 0.1;
        let factor = 1.0 - (1.0 - lerp_base).powf(dt * 60.0);
        let factor = factor.clamp(0.0, 1.0);

        self.position = self.position.lerp(self.target_position, factor);
        self.zoom = self.zoom + (self.target_zoom - self.zoom) * factor;
    }

    /// Adjust target zoom to zoom in (show less world space).
    pub fn zoom_in(&mut self) {
        self.target_zoom = (self.target_zoom / 1.05).max(self.min_zoom);
    }

    /// Adjust target zoom to zoom out (show more world space).
    pub fn zoom_out(&mut self) {
        self.target_zoom = (self.target_zoom * 1.05).min(self.max_zoom);
    }

    /// Convert screen coordinates (pixels, origin top-left) to world coordinates.
    pub fn screen_to_world(&self, screen_x: f32, screen_y: f32) -> Vec2 {
        // Normalized device coords: center of viewport = (0, 0)
        let ndc_x = (screen_x / self.viewport_width) * 2.0 - 1.0;
        let ndc_y = -((screen_y / self.viewport_height) * 2.0 - 1.0); // flip Y

        let aspect = self.viewport_width as f64 / self.viewport_height as f64;
        let half_w = self.zoom;
        let half_h = self.zoom / aspect;

        Vec2::new(
            self.position.x + ndc_x as f64 * half_w,
            self.position.y + ndc_y as f64 * half_h,
        )
    }

    /// Convert world coordinates to screen coordinates (pixels, origin top-left).
    pub fn world_to_screen(&self, world: Vec2) -> (f32, f32) {
        let aspect = self.viewport_width as f64 / self.viewport_height as f64;
        let half_w = self.zoom;
        let half_h = self.zoom / aspect;

        let ndc_x = (world.x - self.position.x) / half_w;
        let ndc_y = (world.y - self.position.y) / half_h;

        let screen_x = ((ndc_x + 1.0) * 0.5) as f32 * self.viewport_width;
        let screen_y = ((-ndc_y + 1.0) * 0.5) as f32 * self.viewport_height;

        (screen_x, screen_y)
    }

    /// Build an orthographic projection matrix for wgpu.
    ///
    /// Maps world coordinates to clip space [-1, 1] on all axes.
    /// Returns a column-major 4x4 matrix suitable for wgpu's coordinate system
    /// (clip space Z in [0, 1]).
    pub fn view_projection_matrix(&self) -> [[f32; 4]; 4] {
        let aspect = self.viewport_width as f64 / self.viewport_height as f64;
        let half_w = self.zoom;
        let half_h = self.zoom / aspect;

        let left = self.position.x - half_w;
        let right = self.position.x + half_w;
        let bottom = self.position.y - half_h;
        let top = self.position.y + half_h;

        // Near/far for 2D (Z range [0, 1] for wgpu).
        let near = 0.0_f64;
        let far = 1.0_f64;

        // Orthographic projection (column-major):
        // Maps [left,right] x [bottom,top] x [near,far] -> [-1,1] x [-1,1] x [0,1]
        let rml = right - left;
        let tmb = top - bottom;
        let fmn = far - near;

        [
            [(2.0 / rml) as f32, 0.0, 0.0, 0.0],
            [0.0, (2.0 / tmb) as f32, 0.0, 0.0],
            [0.0, 0.0, (1.0 / fmn) as f32, 0.0],
            [
                (-(right + left) / rml) as f32,
                (-(top + bottom) / tmb) as f32,
                (-near / fmn) as f32,
                1.0,
            ],
        ]
    }

    /// Update viewport dimensions (e.g., on window resize).
    pub fn resize(&mut self, width: f32, height: f32) {
        self.viewport_width = width;
        self.viewport_height = height;
    }
}
