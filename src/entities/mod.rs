pub mod black_hole;
pub mod ship;
pub mod projectile;
pub mod bot;
pub mod bot_archetypes;
pub mod effects;

pub use black_hole::BlackHole;
pub use ship::{PlayerShip, ThrustDirection};
pub use projectile::{Projectile, ProjectileType};
pub use bot::{Bot, BotArchetype, BotGoal};
pub use bot_archetypes::{ArchetypeStats, get_archetype_stats};
pub use effects::{Explosion, ParticleEffect, spawn_explosion_particles, spawn_thrust_particle, spawn_spaghettification_particles};

use std::sync::atomic::{AtomicU64, Ordering};

pub type EntityId = u64;

static ENTITY_ID_COUNTER: AtomicU64 = AtomicU64::new(1);

pub fn next_entity_id() -> EntityId {
    ENTITY_ID_COUNTER.fetch_add(1, Ordering::Relaxed)
}
