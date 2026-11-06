#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DetectedColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
    pub hex: String,
}

pub fn detect_color(input: &str) -> Option<DetectedColor> {
    let s = input.trim();
    if s.is_empty() || s.len() > 64 {
        return None;
    }

    if let Some(c) = parse_hex_color(s) {
        return Some(c);
    }

    if let Some(c) = parse_rgb_fn(s) {
        return Some(c);
    }

    if let Some(c) = parse_hsl_fn(s) {
        return Some(c);
    }

    None
}

fn parse_hex_color(s: &str) -> Option<DetectedColor> {
    let hex = s.strip_prefix('#')?;
    let len = hex.len();
    if len != 3 && len != 4 && len != 6 && len != 8 {
        return None;
    }
    if !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }

    let (r, g, b, a) = match len {
        3 => {
            let r = u8::from_str_radix(&hex[0..1], 16).ok()?;
            let g = u8::from_str_radix(&hex[1..2], 16).ok()?;
            let b = u8::from_str_radix(&hex[2..3], 16).ok()?;
            (r * 17, g * 17, b * 17, 255)
        }
        4 => {
            let r = u8::from_str_radix(&hex[0..1], 16).ok()?;
            let g = u8::from_str_radix(&hex[1..2], 16).ok()?;
            let b = u8::from_str_radix(&hex[2..3], 16).ok()?;
            let a = u8::from_str_radix(&hex[3..4], 16).ok()?;
            (r * 17, g * 17, b * 17, a * 17)
        }
        6 => {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            (r, g, b, 255)
        }
        8 => {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            let a = u8::from_str_radix(&hex[6..8], 16).ok()?;
            (r, g, b, a)
        }
        _ => return None,
    };

    let canonical_hex = if a == 255 {
        format!("#{:02X}{:02X}{:02X}", r, g, b)
    } else {
        format!("#{:02X}{:02X}{:02X}{:02X}", r, g, b, a)
    };

    Some(DetectedColor {
        r,
        g,
        b,
        a,
        hex: canonical_hex,
    })
}

fn parse_rgb_fn(s: &str) -> Option<DetectedColor> {
    let lower = s.to_ascii_lowercase();
    let body = lower
        .strip_prefix("rgb(")
        .or_else(|| lower.strip_prefix("rgba("))?;
    let body = body.strip_suffix(')')?.trim();

    let parts: Vec<&str> = body.split(',').map(|p| p.trim()).collect();
    if parts.len() != 3 && parts.len() != 4 {
        return None;
    }

    let r: u8 = parts[0].parse().ok()?;
    let g: u8 = parts[1].parse().ok()?;
    let b: u8 = parts[2].parse().ok()?;
    let a: u8 = if parts.len() == 4 {
        let alpha_f: f32 = parts[3].parse().ok()?;
        (alpha_f.clamp(0.0, 1.0) * 255.0).round() as u8
    } else {
        255
    };

    let canonical_hex = if a == 255 {
        format!("#{:02X}{:02X}{:02X}", r, g, b)
    } else {
        format!("#{:02X}{:02X}{:02X}{:02X}", r, g, b, a)
    };

    Some(DetectedColor {
        r,
        g,
        b,
        a,
        hex: canonical_hex,
    })
}

fn parse_hsl_fn(s: &str) -> Option<DetectedColor> {
    let lower = s.to_ascii_lowercase();
    let body = lower
        .strip_prefix("hsl(")
        .or_else(|| lower.strip_prefix("hsla("))?;
    let body = body.strip_suffix(')')?.trim();

    let parts: Vec<&str> = body.split(',').map(|p| p.trim()).collect();
    if parts.len() != 3 && parts.len() != 4 {
        return None;
    }

    let h: f32 = parts[0].parse().ok()?;
    let s: f32 = parts[1].strip_suffix('%')?.parse().ok()?;
    let l: f32 = parts[2].strip_suffix('%')?.parse().ok()?;
    let a: u8 = if parts.len() == 4 {
        let alpha_f: f32 = parts[3].parse().ok()?;
        (alpha_f.clamp(0.0, 1.0) * 255.0).round() as u8
    } else {
        255
    };

    let (r, g, b) = hsl_to_rgb(
        h % 360.0,
        (s / 100.0).clamp(0.0, 1.0),
        (l / 100.0).clamp(0.0, 1.0),
    );
    let canonical_hex = if a == 255 {
        format!("#{:02X}{:02X}{:02X}", r, g, b)
    } else {
        format!("#{:02X}{:02X}{:02X}{:02X}", r, g, b, a)
    };

    Some(DetectedColor {
        r,
        g,
        b,
        a,
        hex: canonical_hex,
    })
}

fn hsl_to_rgb(h: f32, s: f32, l: f32) -> (u8, u8, u8) {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = l - c / 2.0;

    let (r1, g1, b1) = if h < 60.0 {
        (c, x, 0.0)
    } else if h < 120.0 {
        (x, c, 0.0)
    } else if h < 180.0 {
        (0.0, c, x)
    } else if h < 240.0 {
        (0.0, x, c)
    } else if h < 300.0 {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };

    (
        ((r1 + m) * 255.0).round() as u8,
        ((g1 + m) * 255.0).round() as u8,
        ((b1 + m) * 255.0).round() as u8,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hex_colors() {
        let c1 = detect_color("#fff").unwrap();
        assert_eq!(c1.hex, "#FFFFFF");
        assert_eq!(c1.r, 255);

        let c2 = detect_color("#b8bb26").unwrap();
        assert_eq!(c2.hex, "#B8BB26");
        assert_eq!(c2.r, 184);
        assert_eq!(c2.g, 187);
        assert_eq!(c2.b, 38);

        let c3 = detect_color("#12345678").unwrap();
        assert_eq!(c3.hex, "#12345678");

        assert!(detect_color("#12").is_none());
        assert!(detect_color("not a color").is_none());
    }

    #[test]
    fn test_rgb_and_hsl() {
        let c1 = detect_color("rgb(255, 0, 128)").unwrap();
        assert_eq!(c1.r, 255);
        assert_eq!(c1.g, 0);
        assert_eq!(c1.b, 128);

        let c2 = detect_color("hsl(0, 100%, 50%)").unwrap();
        assert_eq!(c2.r, 255);
        assert_eq!(c2.g, 0);
        assert_eq!(c2.b, 0);
    }
}
