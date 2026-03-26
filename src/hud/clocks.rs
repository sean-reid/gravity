use crate::rendering::hud_render::HudElement;
use crate::util::Color;

fn format_time(seconds: f64) -> String {
    let total_tenths = (seconds * 10.0).floor() as u64;
    let minutes = total_tenths / 600;
    let secs = (total_tenths % 600) / 10;
    let tenths = total_tenths % 10;
    format!("{:02}:{:02}.{}", minutes, secs, tenths)
}

pub fn build_clock_elements(coord_time: f64, proper_time: f64, _vw: f32, _vh: f32, s: f32) -> Vec<HudElement> {
    let x = 16.0 * s;
    let scale = 2.0 * s;
    let line_h = 14.0 * scale;

    let coord_str = format!("COORD  {}", format_time(coord_time));
    let proper_str = format!("PROPER {}", format_time(proper_time));

    vec![
        HudElement::Text {
            x,
            y: 16.0 * s,
            text: coord_str,
            color: Color::WHITE.to_array(),
            scale,
        },
        HudElement::Text {
            x,
            y: 16.0 * s + line_h + 4.0 * s,
            text: proper_str,
            color: Color::CYAN.to_array(),
            scale,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_time_zero() {
        assert_eq!(format_time(0.0), "00:00.0");
    }

    #[test]
    fn test_format_time_90_5() {
        assert_eq!(format_time(90.5), "01:30.5");
    }

    #[test]
    fn test_build_clock_elements_count() {
        let elems = build_clock_elements(10.0, 8.5, 1280.0, 720.0, 1.0);
        assert_eq!(elems.len(), 2);
    }
}
