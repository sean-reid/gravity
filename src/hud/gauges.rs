use crate::rendering::hud_render::HudElement;
use crate::util::Color;

pub fn build_gauge_elements(health: f64, shields: f64, fuel: f64, _vw: f32, vh: f32, s: f32) -> Vec<HudElement> {
    let bar_width = 200.0 * s;
    let bar_height = 20.0 * s;
    let bar_spacing = 32.0 * s;
    let margin_left = 16.0 * s;
    let margin_bottom = 24.0 * s;
    let text_scale = 2.0 * s;
    let label_offset = 50.0 * s;

    let mut elements = Vec::with_capacity(12);
    let base_y = vh - margin_bottom - bar_height;

    let gauges: [(&str, f64, Color); 3] = [
        ("HP", health, Color::RED),
        ("SH", shields, Color::BLUE),
        ("FL", fuel, Color::GREEN),
    ];

    for (i, (label, value, fill_color)) in gauges.iter().enumerate() {
        let y = base_y - (2 - i) as f32 * bar_spacing;
        let fraction = (*value as f32 / 100.0).clamp(0.0, 1.0);
        let bar_x = margin_left + label_offset;

        elements.push(HudElement::Text {
            x: margin_left,
            y: y + 1.0,
            text: label.to_string(),
            color: Color::WHITE.to_array(),
            scale: text_scale,
        });

        elements.push(HudElement::Rect {
            x: bar_x, y, w: bar_width, h: bar_height,
            color: fill_color.with_alpha(0.35).to_array(),
        });

        let fill_w = bar_width * fraction;
        if fill_w > 0.0 {
            elements.push(HudElement::Rect {
                x: bar_x, y, w: fill_w, h: bar_height,
                color: fill_color.to_array(),
            });
        }

        elements.push(HudElement::Text {
            x: bar_x + bar_width + 8.0 * s,
            y: y + 1.0,
            text: format!("{}", *value as i32),
            color: Color::WHITE.to_array(),
            scale: text_scale,
        });
    }

    elements
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_gauge_elements_count() {
        let elems = build_gauge_elements(100.0, 50.0, 25.0, 1280.0, 720.0, 1.0);
        assert_eq!(elems.len(), 12);
    }

    #[test]
    fn test_build_gauge_zero_fill() {
        let elems = build_gauge_elements(0.0, 0.0, 0.0, 1280.0, 720.0, 1.0);
        assert_eq!(elems.len(), 9);
    }
}
