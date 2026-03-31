use crate::util::Vec2;
use super::{EntityId, next_entity_id};
use super::bot_archetypes::get_archetype_stats;

pub const BOT_RADIUS: f64 = 0.15;
pub const BOT_SHIELD_REGEN_DELAY: f64 = 2.0;
pub const BOT_MAX_TRAIL_LENGTH: usize = 60;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BotArchetype {
    Skirmisher,
    Diver,
    Vulture,
    Anchor,
    Swarm,
    Commander,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BotGoal {
    Orbit,
    Attack,
    Dive,
    Climb,
    Retreat,
    FormationHold(Vec2),
    Guard,
}

impl Default for BotGoal {
    fn default() -> Self {
        BotGoal::Orbit
    }
}

#[derive(Debug, Clone)]
pub struct Bot {
    pub id: EntityId,
    pub archetype: BotArchetype,
    pub position: Vec2,
    pub velocity: Vec2,
    pub acceleration: Vec2,
    pub health: f64,
    pub max_health: f64,
    pub shields: f64,
    pub max_shields: f64,
    pub fuel: f64,
    pub max_fuel: f64,
    pub tau: f64,
    pub proper_time: f64,
    pub decision_interval: f64,
    pub time_since_last_decision: f64,
    pub turret_angle: f64,
    pub current_goal: BotGoal,
    pub target: Option<EntityId>,
    pub weapon_cooldown: f64,
    pub shield_regen_delay_timer: f64,
    pub shield_regen_rate: f64,
    pub alive: bool,
    pub trail: Vec<Vec2>,
    pub preferred_altitude: f64,
    // Swarm group coordination
    pub swarm_group_id: Option<u32>,
    pub formation_slot: Option<Vec2>,
}

impl Bot {
    pub fn new(
        archetype: BotArchetype,
        position: Vec2,
        velocity: Vec2,
        difficulty_scale: f64,
    ) -> Self {
        let stats = get_archetype_stats(&archetype, difficulty_scale);
        Self {
            id: next_entity_id(),
            archetype,
            position,
            velocity,
            acceleration: Vec2::ZERO,
            health: stats.health,
            max_health: stats.health,
            shields: stats.shields,
            max_shields: stats.shields,
            fuel: stats.fuel,
            max_fuel: stats.fuel,
            tau: 1.0,
            proper_time: 0.0,
            decision_interval: stats.decision_interval,
            time_since_last_decision: 0.0,
            turret_angle: 0.0,
            current_goal: BotGoal::Orbit,
            target: None,
            weapon_cooldown: 0.0,
            shield_regen_delay_timer: BOT_SHIELD_REGEN_DELAY, // start ready to regen
            shield_regen_rate: stats.shield_regen_rate,
            alive: true,
            trail: Vec::with_capacity(BOT_MAX_TRAIL_LENGTH),
            preferred_altitude: stats.preferred_altitude,
            swarm_group_id: None,
            formation_slot: None,
        }
    }

    /// Apply damage, absorbed first by shields then health.
    pub fn apply_damage(&mut self, amount: f64) {
        self.shield_regen_delay_timer = 0.0;

        if self.shields > 0.0 {
            let shield_absorbed = amount.min(self.shields);
            self.shields -= shield_absorbed;
            let remaining = amount - shield_absorbed;
            if remaining > 0.0 {
                self.health = (self.health - remaining).max(0.0);
            }
        } else {
            self.health = (self.health - amount).max(0.0);
        }

        if self.health <= 0.0 {
            self.alive = false;
        }
    }

    /// Regenerate shields over proper time.
    pub fn regenerate(&mut self, dt_proper: f64) {
        self.shield_regen_delay_timer += dt_proper;
        if self.shield_regen_delay_timer >= BOT_SHIELD_REGEN_DELAY && self.shield_regen_rate > 0.0 {
            self.shields = (self.shields + self.shield_regen_rate * dt_proper).min(self.max_shields);
        }
    }

    pub fn is_dead(&self) -> bool {
        !self.alive || self.health <= 0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_skirmisher() {
        let bot = Bot::new(BotArchetype::Skirmisher, Vec2::new(8.0, 0.0), Vec2::ZERO, 0.0);
        assert!((bot.health - 28.0).abs() < 1e-10); // 40 * 0.7 at d=0
        assert!((bot.shields - 21.0).abs() < 1e-10); // 30 * 0.7 at d=0
        assert!(bot.alive);
    }

    #[test]
    fn test_new_commander_with_difficulty() {
        let bot = Bot::new(BotArchetype::Commander, Vec2::new(10.0, 0.0), Vec2::ZERO, 1.0);
        assert!((bot.health - 300.0).abs() < 1e-10); // 200 * 1.5
        assert!((bot.shields - 225.0).abs() < 1e-10); // 150 * 1.5
    }

    #[test]
    fn test_bot_damage_shields_first() {
        let mut bot = Bot::new(BotArchetype::Diver, Vec2::ZERO, Vec2::ZERO, 0.0);
        bot.apply_damage(20.0);
        assert!((bot.shields - 8.0).abs() < 1e-10); // 28 - 20 at d=0 (40*0.7)
        assert!((bot.health - 35.0).abs() < 1e-10); // 50*0.7 at d=0
    }

    #[test]
    fn test_bot_lethal() {
        let mut bot = Bot::new(BotArchetype::Swarm, Vec2::ZERO, Vec2::ZERO, 0.0);
        // Swarm: 15 hp, 0 shields
        bot.apply_damage(20.0);
        assert!(bot.is_dead());
    }

    #[test]
    fn test_bot_regenerate() {
        let mut bot = Bot::new(BotArchetype::Skirmisher, Vec2::ZERO, Vec2::ZERO, 0.0);
        bot.shields = 10.0;
        bot.shield_regen_delay_timer = BOT_SHIELD_REGEN_DELAY;
        bot.regenerate(1.0);
        // shield_regen_rate for Skirmisher = 3.0
        assert!((bot.shields - 13.0).abs() < 1e-10);
    }

    #[test]
    fn test_swarm_no_regen() {
        let mut bot = Bot::new(BotArchetype::Swarm, Vec2::ZERO, Vec2::ZERO, 0.0);
        bot.shields = 0.0;
        bot.shield_regen_delay_timer = 100.0;
        bot.regenerate(10.0);
        assert!((bot.shields - 0.0).abs() < 1e-10);
    }
}
