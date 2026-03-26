use std::ops::{Add, AddAssign, Sub, SubAssign, Mul, MulAssign, Div, Neg};

#[derive(Debug, Clone, Copy, PartialEq, Default, serde::Serialize, serde::Deserialize)]
pub struct Vec2 {
    pub x: f64,
    pub y: f64,
}

impl Vec2 {
    pub const ZERO: Vec2 = Vec2 { x: 0.0, y: 0.0 };

    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    pub fn from_angle(angle: f64) -> Self {
        Self {
            x: angle.cos(),
            y: angle.sin(),
        }
    }

    pub fn length_squared(self) -> f64 {
        self.x * self.x + self.y * self.y
    }

    pub fn length(self) -> f64 {
        self.length_squared().sqrt()
    }

    pub fn normalized(self) -> Self {
        let len = self.length();
        if len < 1e-12 {
            Self::ZERO
        } else {
            self / len
        }
    }

    pub fn dot(self, other: Self) -> f64 {
        self.x * other.x + self.y * other.y
    }

    pub fn cross(self, other: Self) -> f64 {
        self.x * other.y - self.y * other.x
    }

    pub fn angle(self) -> f64 {
        self.y.atan2(self.x)
    }

    pub fn distance(self, other: Self) -> f64 {
        (self - other).length()
    }

    pub fn distance_squared(self, other: Self) -> f64 {
        (self - other).length_squared()
    }

    pub fn perpendicular(self) -> Self {
        Self { x: -self.y, y: self.x }
    }

    pub fn rotate(self, angle: f64) -> Self {
        let c = angle.cos();
        let s = angle.sin();
        Self {
            x: self.x * c - self.y * s,
            y: self.x * s + self.y * c,
        }
    }

    pub fn lerp(self, other: Self, t: f64) -> Self {
        self * (1.0 - t) + other * t
    }

    pub fn reflect(self, normal: Self) -> Self {
        self - normal * 2.0 * self.dot(normal)
    }

    pub fn as_f32_array(self) -> [f32; 2] {
        [self.x as f32, self.y as f32]
    }
}

impl Add for Vec2 {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self { x: self.x + rhs.x, y: self.y + rhs.y }
    }
}

impl AddAssign for Vec2 {
    fn add_assign(&mut self, rhs: Self) {
        self.x += rhs.x;
        self.y += rhs.y;
    }
}

impl Sub for Vec2 {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Self { x: self.x - rhs.x, y: self.y - rhs.y }
    }
}

impl SubAssign for Vec2 {
    fn sub_assign(&mut self, rhs: Self) {
        self.x -= rhs.x;
        self.y -= rhs.y;
    }
}

impl Mul<f64> for Vec2 {
    type Output = Self;
    fn mul(self, rhs: f64) -> Self {
        Self { x: self.x * rhs, y: self.y * rhs }
    }
}

impl Mul<Vec2> for f64 {
    type Output = Vec2;
    fn mul(self, rhs: Vec2) -> Vec2 {
        Vec2 { x: self * rhs.x, y: self * rhs.y }
    }
}

impl MulAssign<f64> for Vec2 {
    fn mul_assign(&mut self, rhs: f64) {
        self.x *= rhs;
        self.y *= rhs;
    }
}

impl Div<f64> for Vec2 {
    type Output = Self;
    fn div(self, rhs: f64) -> Self {
        Self { x: self.x / rhs, y: self.y / rhs }
    }
}

impl Neg for Vec2 {
    type Output = Self;
    fn neg(self) -> Self {
        Self { x: -self.x, y: -self.y }
    }
}
