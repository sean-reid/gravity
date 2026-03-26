use crate::rendering::hud_render::HudElement;
use crate::util::Color;

pub fn build_tempo_elements(tempo: f64, vw: f32, _vh: f32, s: f32) -> Vec<HudElement> {
    let label = format!("TEMPO: {:.1}x", tempo);
    let color = Color::tempo_color(tempo as f32);
    let scale = 2.2 * s;

    let char_w = 8.0 * scale;
    let approx_width = label.len() as f32 * char_w;
    let x = vw - approx_width - 16.0 * s;
    let y = 16.0 * s;

    vec![HudElement::Text {
        x,
        y,
        text: label,
        color: color.to_array(),
        scale,
    }]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_tempo_elements_count() {
        let elems = build_tempo_elements(1.0, 1280.0, 720.0, 1.0);
        assert_eq!(elems.len(), 1);
    }
}
