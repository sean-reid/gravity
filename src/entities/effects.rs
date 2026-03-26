use crate::util::Vec2;
use crate::util::Color;

/// Default explosion duration in seconds.
pub const DEFAULT_EXPLOSION_DURATION: f64 = 0.5;

#[derive(Debug, Clone)]
pub struct Explosion {
    pub position: Vec2,
    pub timer: f64,
    pub duration: f64,
    pub radius: f64,
    pub color: Color,
}

impl Explosion {
    pub fn new(position: Vec2, radius: f64, color: Color) -> Self {
        Self {
            position,
            timer: 0.0,
            duration: DEFAULT_EXPLOSION_DURATION,
            radius,
            color,
        }
    }

    pub fn is_finished(&self) -> bool {
        self.timer >= self.duration
    }

    /// Returns progress in [0, 1].
    pub fn progress(&self) -> f64 {
        (self.timer / self.duration).clamp(0.0, 1.0)
    }
}

#[derive(Debug, Clone)]
pub struct ParticleEffect {
    pub position: Vec2,
    pub velocity: Vec2,
    pub color: Color,
    pub size: f32,
    pub lifetime: f64,
    pub age: f64,
}

impl ParticleEffect {
    pub fn is_alive(&self) -> bool {
        self.age < self.lifetime
    }

    /// Returns progress in [0, 1].
    pub fn progress(&self) -> f64 {
        (self.age / self.lifetime).clamp(0.0, 1.0)
    }

    /// Returns alpha that fades out over lifetime.
    pub fn alpha(&self) -> f32 {
        (1.0 - self.progress() as f32).max(0.0)
    }
}

/// Spawn a burst of particles for an explosion effect.
pub fn spawn_explosion_particles(
    pos: Vec2,
    base_vel: Vec2,
    color: Color,
    count: usize,
) -> Vec<ParticleEffect> {
    let mut particles = Vec::with_capacity(count);
    let angle_step = std::f64::consts::TAU / count as f64;

    for i in 0..count {
        let angle = angle_step * i as f64;
        // Vary speed between 2 and 6 using a simple deterministic pattern
        let speed = 2.0 + 4.0 * ((i as f64 * 0.618).fract());
        let dir = Vec2::from_angle(angle);
        let vel = base_vel * 0.3 + dir * speed;
        let size_variation = 0.02 + 0.03 * ((i as f64 * 0.382).fract()) as f32;

        particles.push(ParticleEffect {
            position: pos,
            velocity: vel,
            color,
            size: size_variation,
            lifetime: 0.3 + 0.4 * (i as f64 * 0.618).fract(),
            age: 0.0,
        });
    }

    particles
}

/// Spawn a single thrust particle behind a ship.
pub fn spawn_thrust_particle(
    ship_pos: Vec2,
    ship_vel: Vec2,
    thrust_dir: Vec2,
) -> ParticleEffect {
    // Particle goes opposite to thrust direction
    let exhaust_dir = -thrust_dir.normalized();
    let vel = ship_vel * 0.5 + exhaust_dir * 3.0;

    ParticleEffect {
        position: ship_pos + exhaust_dir * 0.2,
        velocity: vel,
        color: Color::ORANGE.with_alpha(0.8),
        size: 0.03,
        lifetime: 0.3,
        age: 0.0,
    }
}

/// Spawn particles for the spaghettification visual effect near a black hole.
pub fn spawn_spaghettification_particles(
    pos: Vec2,
    bh_pos: Vec2,
    count: usize,
) -> Vec<ParticleEffect> {
    let mut particles = Vec::with_capacity(count);
    let to_bh = (bh_pos - pos).normalized();

    for i in 0..count {
        // Particles stretch toward the black hole
        let frac = i as f64 / count as f64;
        let lateral_offset = (frac * std::f64::consts::TAU).sin() * 0.3;
        let perpendicular = to_bh.perpendicular();

        let vel = to_bh * (5.0 + 10.0 * frac) + perpendicular * lateral_offset;

        particles.push(ParticleEffect {
            position: pos,
            velocity: vel,
            color: Color::RED.with_alpha(0.6),
            size: 0.02 + 0.02 * frac as f32,
            lifetime: 0.4 + 0.3 * frac,
            age: 0.0,
        });
    }

    particles
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_explosion_lifecycle() {
        let mut exp = Explosion::new(Vec2::ZERO, 1.0, Color::RED);
        assert!(!exp.is_finished());
        exp.timer = exp.duration;
        assert!(exp.is_finished());
    }

    #[test]
    fn test_particle_lifecycle() {
        let p = ParticleEffect {
            position: Vec2::ZERO,
            velocity: Vec2::new(1.0, 0.0),
            color: Color::WHITE,
            size: 0.05,
            lifetime: 1.0,
            age: 0.0,
        };
        assert!(p.is_alive());
        assert!((p.alpha() - 1.0).abs() < 1e-5);
    }

    #[test]
    fn test_particle_fades() {
        let p = ParticleEffect {
            position: Vec2::ZERO,
            velocity: Vec2::ZERO,
            color: Color::WHITE,
            size: 0.05,
            lifetime: 1.0,
            age: 0.5,
        };
        assert!((p.alpha() - 0.5).abs() < 1e-5);
    }

    #[test]
    fn test_spawn_explosion_particles_count() {
        let particles = spawn_explosion_particles(Vec2::ZERO, Vec2::ZERO, Color::RED, 12);
        assert_eq!(particles.len(), 12);
    }

    #[test]
    fn test_spawn_thrust_particle() {
        let p = spawn_thrust_particle(
            Vec2::new(5.0, 0.0),
            Vec2::new(0.0, 2.0),
            Vec2::new(1.0, 0.0),
        );
        // Exhaust should go in -x direction
        assert!(p.velocity.x < 0.0);
    }

    #[test]
    fn test_spawn_spaghettification() {
        let particles = spawn_spaghettification_particles(
            Vec2::new(2.0, 0.0),
            Vec2::ZERO,
            8,
        );
        assert_eq!(particles.len(), 8);
        // Particles should be moving generally toward the BH (negative x)
        for p in &particles {
            assert!(p.velocity.x < 0.0);
        }
    }
}
