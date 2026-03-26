use crate::entities::bot::BotArchetype;
use crate::rendering::hud_render::HudElement;
use crate::util::Color;

fn archetype_letter(archetype: &BotArchetype) -> &'static str {
    match archetype {
        BotArchetype::Skirmisher => "S",
        BotArchetype::Diver => "D",
        BotArchetype::Vulture => "V",
        BotArchetype::Anchor => "A",
        BotArchetype::Swarm => "W",
        BotArchetype::Commander => "C",
    }
}

fn archetype_color(archetype: &BotArchetype) -> Color {
    match archetype {
        BotArchetype::Skirmisher => Color::skirmisher(),
        BotArchetype::Diver => Color::diver(),
        BotArchetype::Vulture => Color::vulture(),
        BotArchetype::Anchor => Color::anchor(),
        BotArchetype::Swarm => Color::swarm(),
        BotArchetype::Commander => Color::commander(),
    }
}

pub fn build_radar_elements(
    bot_depths: &[(BotArchetype, f64)],
    vw: f32, vh: f32, s: f32,
) -> Vec<HudElement> {
    if bot_depths.is_empty() {
        return Vec::new();
    }

    let strip_w = 120.0 * s;
    let strip_h = 250.0 * s;
    let margin_r = 16.0 * s;
    let margin_b = 20.0 * s;
    let strip_x = vw - margin_r - strip_w;
    let strip_top = vh - margin_b - strip_h;

    let mut elements = Vec::with_capacity(bot_depths.len() * 3 + 2);

    elements.push(HudElement::Rect {
        x: strip_x, y: strip_top, w: strip_w, h: strip_h,
        color: Color::WHITE.with_alpha(0.08).to_array(),
    });

    let label_scale = 1.6 * s;
    elements.push(HudElement::Text {
        x: strip_x + 16.0 * s,
        y: strip_top + 6.0 * s,
        text: "DEPTH".to_string(),
        color: Color::WHITE.with_alpha(0.6).to_array(),
        scale: label_scale,
    });

    let usable_top = strip_top + 28.0 * s;
    let usable_height = strip_h - 36.0 * s;
    let letter_scale = 2.0 * s;
    let ratio_scale = 1.5 * s;

    for &(ref archetype, tau_ratio) in bot_depths {
        let clamped = (tau_ratio as f32).clamp(0.2, 2.0);
        let normalized = 1.0 - (clamped - 0.2) / 1.8;
        let y = usable_top + normalized * usable_height;

        let color = archetype_color(archetype);

        elements.push(HudElement::Text {
            x: strip_x + 8.0 * s, y,
            text: archetype_letter(archetype).to_string(),
            color: color.to_array(),
            scale: letter_scale,
        });

        elements.push(HudElement::Text {
            x: strip_x + 30.0 * s, y,
            text: format!("{:.1}x", tau_ratio),
            color: color.with_alpha(0.8).to_array(),
            scale: ratio_scale,
        });
    }

    elements
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_radar() {
        let elems = build_radar_elements(&[], 1280.0, 720.0, 1.0);
        assert!(elems.is_empty());
    }

    #[test]
    fn test_single_bot_radar() {
        let elems = build_radar_elements(&[(BotArchetype::Skirmisher, 1.0)], 1280.0, 720.0, 1.0);
        assert_eq!(elems.len(), 4);
    }
}
