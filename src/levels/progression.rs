use crate::weapons::WeaponType;

/// Tracks the player's progression through levels, weapon unlocks, and ability unlocks.
#[derive(Debug, Clone)]
pub struct Progression {
    pub highest_level: u32,
    pub unlocked_weapons: Vec<WeaponType>,
    pub unlocked_orbit_anchor: bool,
    pub unlocked_tidal_flare: bool,
}

impl Progression {
    /// Create a new progression state at level 0 with only the Railgun unlocked.
    pub fn new() -> Self {
        Self {
            highest_level: 0,
            unlocked_weapons: vec![WeaponType::Railgun],
            unlocked_orbit_anchor: false,
            unlocked_tidal_flare: false,
        }
    }

    /// Unlock weapons and abilities for the given level WITHOUT advancing highest_level.
    /// Call this when starting a level so the player has access to their unlocks.
    pub fn unlock_for_level(&mut self, level: u32) {
        // Weapon unlock thresholds
        let weapon_unlocks: &[(u32, WeaponType)] = &[
            (1, WeaponType::Railgun),
            (3, WeaponType::MassDriver),
            (6, WeaponType::PhotonLance),
            (9, WeaponType::GravityBomb),
            (12, WeaponType::ImpulseRocket),
            (15, WeaponType::TidalMine),
        ];

        for &(threshold, weapon) in weapon_unlocks {
            if level >= threshold && !self.unlocked_weapons.contains(&weapon) {
                self.unlocked_weapons.push(weapon);
            }
        }

        // Ability unlocks
        if level >= 8 {
            self.unlocked_orbit_anchor = true;
        }
        if level >= 20 {
            self.unlocked_tidal_flare = true;
        }
    }

    /// Advance to the given level, unlocking weapons/abilities AND setting highest_level.
    /// Call this only when a level is actually completed.
    pub fn advance_to_level(&mut self, level: u32) {
        if level > self.highest_level {
            self.highest_level = level;
        }
        self.unlock_for_level(level);
    }

    /// Check whether a specific weapon is unlocked.
    pub fn has_weapon(&self, weapon: WeaponType) -> bool {
        self.unlocked_weapons.contains(&weapon)
    }

    /// Restore progression from a save state (list of weapon names and flags).
    pub fn from_save(
        highest_level: u32,
        weapon_names: &[String],
        orbit_anchor: bool,
        tidal_flare: bool,
    ) -> Self {
        let unlocked_weapons = weapon_names
            .iter()
            .filter_map(|name| match name.as_str() {
                "Railgun" => Some(WeaponType::Railgun),
                "MassDriver" => Some(WeaponType::MassDriver),
                "PhotonLance" => Some(WeaponType::PhotonLance),
                "GravityBomb" => Some(WeaponType::GravityBomb),
                "ImpulseRocket" => Some(WeaponType::ImpulseRocket),
                "TidalMine" => Some(WeaponType::TidalMine),
                _ => None,
            })
            .collect();

        Self {
            highest_level,
            unlocked_weapons,
            unlocked_orbit_anchor: orbit_anchor,
            unlocked_tidal_flare: tidal_flare,
        }
    }

    /// Serialize weapon list to string names for saving.
    pub fn weapon_names(&self) -> Vec<String> {
        self.unlocked_weapons
            .iter()
            .map(|w| match w {
                WeaponType::Railgun => "Railgun".to_string(),
                WeaponType::MassDriver => "MassDriver".to_string(),
                WeaponType::PhotonLance => "PhotonLance".to_string(),
                WeaponType::GravityBomb => "GravityBomb".to_string(),
                WeaponType::ImpulseRocket => "ImpulseRocket".to_string(),
                WeaponType::TidalMine => "TidalMine".to_string(),
            })
            .collect()
    }
}

impl Default for Progression {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_progression_has_railgun_only() {
        let p = Progression::new();
        assert_eq!(p.highest_level, 0);
        assert_eq!(p.unlocked_weapons.len(), 1);
        assert!(p.has_weapon(WeaponType::Railgun));
        assert!(!p.unlocked_orbit_anchor);
        assert!(!p.unlocked_tidal_flare);
    }

    #[test]
    fn advance_unlocks_weapons() {
        let mut p = Progression::new();
        p.advance_to_level(6);
        assert!(p.has_weapon(WeaponType::Railgun));
        assert!(p.has_weapon(WeaponType::MassDriver));
        assert!(p.has_weapon(WeaponType::PhotonLance));
        assert!(!p.has_weapon(WeaponType::GravityBomb));
    }

    #[test]
    fn advance_unlocks_abilities() {
        let mut p = Progression::new();
        p.advance_to_level(8);
        assert!(p.unlocked_orbit_anchor);
        assert!(!p.unlocked_tidal_flare);

        p.advance_to_level(20);
        assert!(p.unlocked_tidal_flare);
    }

    #[test]
    fn roundtrip_save() {
        let mut p = Progression::new();
        p.advance_to_level(15);

        let names = p.weapon_names();
        let restored = Progression::from_save(
            p.highest_level,
            &names,
            p.unlocked_orbit_anchor,
            p.unlocked_tidal_flare,
        );
        assert_eq!(restored.highest_level, 15);
        assert_eq!(restored.unlocked_weapons.len(), p.unlocked_weapons.len());
    }
}
