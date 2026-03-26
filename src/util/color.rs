/// RGBA color with f32 components in [0, 1]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    pub const WHITE: Color = Color { r: 1.0, g: 1.0, b: 1.0, a: 1.0 };
    pub const BLACK: Color = Color { r: 0.0, g: 0.0, b: 0.0, a: 1.0 };
    pub const RED: Color = Color { r: 1.0, g: 0.2, b: 0.2, a: 1.0 };
    pub const GREEN: Color = Color { r: 0.2, g: 1.0, b: 0.2, a: 1.0 };
    pub const BLUE: Color = Color { r: 0.3, g: 0.5, b: 1.0, a: 1.0 };
    pub const YELLOW: Color = Color { r: 1.0, g: 0.9, b: 0.2, a: 1.0 };
    pub const ORANGE: Color = Color { r: 1.0, g: 0.6, b: 0.1, a: 1.0 };
    pub const CYAN: Color = Color { r: 0.2, g: 0.9, b: 1.0, a: 1.0 };
    pub const MAGENTA: Color = Color { r: 1.0, g: 0.2, b: 0.8, a: 1.0 };
    pub const DIM_RED: Color = Color { r: 0.6, g: 0.15, b: 0.15, a: 1.0 };
    pub const TRANSPARENT: Color = Color { r: 0.0, g: 0.0, b: 0.0, a: 0.0 };

    pub const fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    pub const fn rgb(r: f32, g: f32, b: f32) -> Self {
        Self { r, g, b, a: 1.0 }
    }

    pub fn with_alpha(self, a: f32) -> Self {
        Self { a, ..self }
    }

    pub fn lerp(self, other: Self, t: f32) -> Self {
        Self {
            r: self.r + (other.r - self.r) * t,
            g: self.g + (other.g - self.g) * t,
            b: self.b + (other.b - self.b) * t,
            a: self.a + (other.a - self.a) * t,
        }
    }

    pub fn to_array(self) -> [f32; 4] {
        [self.r, self.g, self.b, self.a]
    }

    pub fn from_array(arr: [f32; 4]) -> Self {
        Self { r: arr[0], g: arr[1], b: arr[2], a: arr[3] }
    }

    /// Apply blueshift (shift toward blue/white)
    pub fn blueshift(self, factor: f32) -> Self {
        let t = factor.clamp(0.0, 1.0);
        Self {
            r: self.r + (0.8 - self.r) * t * 0.5,
            g: self.g + (0.9 - self.g) * t * 0.5,
            b: self.b + (1.0 - self.b) * t,
            a: self.a,
        }
    }

    /// Apply redshift (shift toward red/orange)
    pub fn redshift(self, factor: f32) -> Self {
        let t = factor.clamp(0.0, 1.0);
        Self {
            r: self.r + (1.0 - self.r) * t,
            g: self.g + (0.3 - self.g) * t * 0.5,
            b: self.b * (1.0 - t * 0.8),
            a: self.a,
        }
    }

    /// Get color for world tempo display: white → yellow → orange → red
    pub fn tempo_color(tempo: f32) -> Self {
        if tempo <= 1.0 {
            Color::WHITE
        } else if tempo <= 1.5 {
            let t = (tempo - 1.0) / 0.5;
            Color::WHITE.lerp(Color::YELLOW, t)
        } else if tempo <= 2.0 {
            let t = (tempo - 1.5) / 0.5;
            Color::YELLOW.lerp(Color::ORANGE, t)
        } else {
            let t = ((tempo - 2.0) / 1.0).min(1.0);
            Color::ORANGE.lerp(Color::RED, t)
        }
    }

    /// Archetype colors
    pub fn skirmisher() -> Self { Color::rgb(0.4, 0.7, 1.0) }
    pub fn diver() -> Self { Color::rgb(1.0, 0.4, 0.2) }
    pub fn vulture() -> Self { Color::rgb(0.6, 0.9, 0.3) }
    pub fn anchor() -> Self { Color::rgb(0.8, 0.5, 0.2) }
    pub fn swarm() -> Self { Color::rgb(0.9, 0.9, 0.3) }
    pub fn commander() -> Self { Color::rgb(0.9, 0.2, 0.9) }
    pub fn player() -> Self { Color::WHITE }
}

impl Default for Color {
    fn default() -> Self {
        Color::WHITE
    }
}
