use crate::rendering::hud_render::HudElement;
use crate::util::Color;

const WEAPON_NAMES: [&str; 6] = [
    "RAILGUN", "MASS DRIVER", "PHOTON LANCE",
    "GRAV BOMB", "IMPULSE ROCKET", "TIDAL MINE",
];

pub fn build_warning_elements(
    escape_vel_warning: bool,
    low_fuel_warning: bool,
    time: f64,
    vw: f32, vh: f32, s: f32,
) -> Vec<HudElement> {
    let mut elements = Vec::new();

    if escape_vel_warning {
        let pulse = ((time * 6.0).sin() * 0.5 + 0.5) as f32;
        let alpha = 0.4 + pulse * 0.6;
        let text = "ESCAPE VELOCITY";
        let scale = 2.5 * s;
        let char_w = 8.0 * scale;
        let approx_width = text.len() as f32 * char_w;
        let x = (vw - approx_width) * 0.5;
        let y = vh - 140.0 * s;

        elements.push(HudElement::Text {
            x, y,
            text: text.to_string(),
            color: Color::RED.with_alpha(alpha).to_array(),
            scale,
        });
    }

    if low_fuel_warning {
        let blink = ((time * 3.0).sin() > -0.4) as u8 as f32;
        if blink > 0.0 {
            let text = "LOW FUEL";
            let scale = 2.0 * s;
            let x = 16.0 * s;
            let y = vh - 120.0 * s;

            elements.push(HudElement::Text {
                x, y,
                text: text.to_string(),
                color: Color::YELLOW.with_alpha(0.9).to_array(),
                scale,
            });
        }
    }

    elements
}

pub fn build_weapon_elements(
    active: usize,
    cooldowns: &[f64; 6],
    available: &[bool; 6],
    vw: f32, vh: f32, s: f32,
) -> Vec<HudElement> {
    let mut elements = Vec::with_capacity(20);

    let slot_w = 36.0 * s;
    let slot_h = 28.0 * s;
    let slot_gap = 6.0 * s;
    let total_slots_width = 6.0 * slot_w + 5.0 * slot_gap;
    let hotbar_x = (vw - total_slots_width) * 0.5;
    let hotbar_y = vh - 55.0 * s;

    // Weapon name
    let name = WEAPON_NAMES[active];
    let name_scale = 2.0 * s;
    let char_w = 8.0 * name_scale;
    let approx_name_width = name.len() as f32 * char_w;
    let name_x = (vw - approx_name_width) * 0.5;
    let name_y = hotbar_y - 40.0 * s;

    elements.push(HudElement::Text {
        x: name_x, y: name_y,
        text: name.to_string(),
        color: Color::WHITE.to_array(),
        scale: name_scale,
    });

    // Cooldown bar
    let cooldown_fraction = cooldowns[active].clamp(0.0, 1.0) as f32;
    let bar_width = 140.0 * s;
    let bar_x = (vw - bar_width) * 0.5;
    let bar_y = name_y + 14.0 * name_scale;
    let bar_h = 5.0 * s;

    elements.push(HudElement::Rect {
        x: bar_x, y: bar_y, w: bar_width, h: bar_h,
        color: Color::WHITE.with_alpha(0.15).to_array(),
    });

    if cooldown_fraction > 0.0 {
        elements.push(HudElement::Rect {
            x: bar_x, y: bar_y, w: bar_width * cooldown_fraction, h: bar_h,
            color: Color::ORANGE.with_alpha(0.7).to_array(),
        });
    }

    // Hotbar slots
    let num_scale = 1.8 * s;
    for i in 0..6 {
        let sx = hotbar_x + i as f32 * (slot_w + slot_gap);

        let bg_color = if !available[i] {
            Color::new(0.2, 0.2, 0.2, 0.5)
        } else if i == active {
            Color::new(0.3, 0.6, 1.0, 0.6)
        } else {
            Color::new(0.3, 0.3, 0.3, 0.4)
        };

        elements.push(HudElement::Rect {
            x: sx, y: hotbar_y, w: slot_w, h: slot_h,
            color: bg_color.to_array(),
        });

        let num_color = if !available[i] {
            Color::new(0.4, 0.4, 0.4, 0.5)
        } else if i == active {
            Color::WHITE
        } else {
            Color::new(0.8, 0.8, 0.8, 0.8)
        };

        elements.push(HudElement::Text {
            x: sx + (slot_w - 8.0 * num_scale) * 0.5,
            y: hotbar_y + (slot_h - 12.0 * num_scale) * 0.5,
            text: format!("{}", i + 1),
            color: num_color.to_array(),
            scale: num_scale,
        });

        if i == active && available[i] {
            let bc = Color::new(0.5, 0.8, 1.0, 0.8);
            elements.push(HudElement::Rect { x: sx, y: hotbar_y, w: slot_w, h: 2.0 * s, color: bc.to_array() });
            elements.push(HudElement::Rect { x: sx, y: hotbar_y + slot_h - 2.0 * s, w: slot_w, h: 2.0 * s, color: bc.to_array() });
        }
    }

    elements
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_warnings() {
        let elems = build_warning_elements(false, false, 0.0, 1280.0, 720.0, 1.0);
        assert!(elems.is_empty());
    }

    #[test]
    fn test_escape_velocity_warning() {
        let elems = build_warning_elements(true, false, 0.0, 1280.0, 720.0, 1.0);
        assert_eq!(elems.len(), 1);
    }

    #[test]
    fn test_weapon_elements_basic() {
        let cooldowns = [0.0; 6];
        let available = [true, true, false, false, false, false];
        let elems = build_weapon_elements(0, &cooldowns, &available, 1280.0, 720.0, 1.0);
        assert!(elems.len() >= 14);
    }
}
