use saturn_core::project::HslQualifier;

/// Converts RGB [0.0, 1.0] to HSL (Hue in [0.0, 360.0], Saturation in [0.0, 1.0], Lightness in [0.0, 1.0]).
pub fn rgb_to_hsl(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = max - min;
    let l = (max + min) / 2.0;

    if delta < 1e-6 {
        return (0.0, 0.0, l);
    }

    let s = if l > 0.5 {
        delta / (2.0 - max - min)
    } else {
        delta / (max + min)
    };

    let mut h = if (max - r).abs() < 1e-6 {
        (g - b) / delta + (if g < b { 6.0 } else { 0.0 })
    } else if (max - g).abs() < 1e-6 {
        (b - r) / delta + 2.0
    } else {
        (r - g) / delta + 4.0
    };

    h *= 60.0;
    (h, s, l)
}

/// Converts HSL (H in [0.0, 360.0], S in [0.0, 1.0], L in [0.0, 1.0]) to RGB in [0.0, 1.0].
pub fn hsl_to_rgb(h: f32, s: f32, l: f32) -> (f32, f32, f32) {
    if s < 1e-6 {
        return (l, l, l);
    }

    let q = if l < 0.5 {
        l * (1.0 + s)
    } else {
        l + s - l * s
    };
    let p = 2.0 * l - q;

    let hk = (h % 360.0 + 360.0) % 360.0 / 360.0;
    let tr = hk + 1.0 / 3.0;
    let tg = hk;
    let tb = hk - 1.0 / 3.0;

    (
        hue_to_rgb(p, q, tr),
        hue_to_rgb(p, q, tg),
        hue_to_rgb(p, q, tb),
    )
}

fn hue_to_rgb(p: f32, q: f32, mut t: f32) -> f32 {
    if t < 0.0 {
        t += 1.0;
    }
    if t > 1.0 {
        t -= 1.0;
    }
    if t < 1.0 / 6.0 {
        return p + (q - p) * 6.0 * t;
    }
    if t < 1.0 / 2.0 {
        return q;
    }
    if t < 2.0 / 3.0 {
        return p + (q - p) * (2.0 / 3.0 - t) * 6.0;
    }
    p
}

/// Computes the matte isolation factor in [0.0, 1.0] for an HSL pixel.
pub fn calculate_matte_factor(h: f32, s: f32, l: f32, qualifier: &HslQualifier) -> f32 {
    // 1. Hue distance (circular modular distance)
    let center_deg = if qualifier.hue_center <= 1.0 {
        (qualifier.hue_center as f32 * 360.0).clamp(0.0, 360.0)
    } else {
        (qualifier.hue_center as f32).clamp(0.0, 360.0)
    };
    let width_deg = if qualifier.hue_width <= 1.0 {
        (qualifier.hue_width as f32 * 360.0).max(1.0)
    } else {
        (qualifier.hue_width as f32).max(1.0)
    };
    let soft_deg = if qualifier.hue_softness <= 1.0 {
        (qualifier.hue_softness as f32 * 360.0).max(0.1)
    } else {
        (qualifier.hue_softness as f32).max(0.1)
    };

    let diff = (h - center_deg).abs();
    let circular_dist = diff.min(360.0 - diff);

    let half_width = width_deg / 2.0;
    let hue_factor = if circular_dist <= half_width {
        1.0
    } else if circular_dist <= half_width + soft_deg {
        1.0 - (circular_dist - half_width) / soft_deg
    } else {
        0.0
    };

    // 2. Saturation factor
    let sat_center = qualifier.sat_center as f32;
    let sat_half_w = (qualifier.sat_width as f32 / 2.0).max(0.01);
    let sat_min = (sat_center - sat_half_w).clamp(0.0, 1.0);
    let sat_max = (sat_center + sat_half_w).clamp(0.0, 1.0);
    let sat_soft = (qualifier.sat_softness as f32).max(0.001);

    let sat_factor = if s >= sat_min && s <= sat_max {
        1.0
    } else if s < sat_min && s >= sat_min - sat_soft {
        (s - (sat_min - sat_soft)) / sat_soft
    } else if s > sat_max && s <= sat_max + sat_soft {
        1.0 - (s - sat_max) / sat_soft
    } else {
        0.0
    };

    // 3. Luminance factor
    let luma_center = qualifier.luma_center as f32;
    let luma_half_w = (qualifier.luma_width as f32 / 2.0).max(0.01);
    let luma_min = (luma_center - luma_half_w).clamp(0.0, 1.0);
    let luma_max = (luma_center + luma_half_w).clamp(0.0, 1.0);
    let luma_soft = (qualifier.luma_softness as f32).max(0.001);

    let luma_factor = if l >= luma_min && l <= luma_max {
        1.0
    } else if l < luma_min && l >= luma_min - luma_soft {
        (l - (luma_min - luma_soft)) / luma_soft
    } else if l > luma_max && l <= luma_max + luma_soft {
        1.0 - (l - luma_max) / luma_soft
    } else {
        0.0
    };

    let mut combined = hue_factor * sat_factor * luma_factor;
    if qualifier.invert {
        combined = 1.0 - combined;
    }
    combined.clamp(0.0, 1.0)
}

/// Generates a float matte array of size width * height for the given image buffer.
pub fn generate_hsl_matte(
    rgba_buf: &[u8],
    width: usize,
    height: usize,
    qualifier: &HslQualifier,
) -> Vec<f32> {
    let mut matte = Vec::with_capacity(width * height);
    for chunk in rgba_buf.chunks_exact(4) {
        let r = chunk[0] as f32 / 255.0;
        let g = chunk[1] as f32 / 255.0;
        let b = chunk[2] as f32 / 255.0;
        let (h, s, l) = rgb_to_hsl(r, g, b);
        matte.push(calculate_matte_factor(h, s, l, qualifier));
    }
    matte
}

/// Applies secondary color grade or displays isolated matte.
pub fn apply_hsl_qualifier_buffer(
    rgba_buf: &mut [u8],
    _width: usize,
    _height: usize,
    qualifier: &HslQualifier,
    show_matte_only: bool,
) {
    let hue_shift = qualifier.hue_shift as f32;
    let sat_boost = qualifier.sat_boost as f32;
    let exp_shift = qualifier.exposure_shift as f32;

    for chunk in rgba_buf.chunks_exact_mut(4) {
        let r = chunk[0] as f32 / 255.0;
        let g = chunk[1] as f32 / 255.0;
        let b = chunk[2] as f32 / 255.0;

        let (h, s, l) = rgb_to_hsl(r, g, b);
        let factor = calculate_matte_factor(h, s, l, qualifier);

        if show_matte_only {
            let val = (factor * 255.0).round().clamp(0.0, 255.0) as u8;
            chunk[0] = val;
            chunk[1] = val;
            chunk[2] = val;
        } else if factor > 1e-4 {
            // Apply secondary adjustment weighted by factor
            let new_h = (h + hue_shift * factor) % 360.0;
            let new_s = (s * (1.0 + (sat_boost - 1.0) * factor)).clamp(0.0, 1.0);
            let new_l = (l + exp_shift * 0.5 * factor).clamp(0.0, 1.0);

            let (target_r, target_g, target_b) = hsl_to_rgb(new_h, new_s, new_l);

            chunk[0] = ((r + (target_r - r) * factor) * 255.0)
                .round()
                .clamp(0.0, 255.0) as u8;
            chunk[1] = ((g + (target_g - g) * factor) * 255.0)
                .round()
                .clamp(0.0, 255.0) as u8;
            chunk[2] = ((b + (target_b - b) * factor) * 255.0)
                .round()
                .clamp(0.0, 255.0) as u8;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rgb_hsl_roundtrip() {
        let (h, s, l) = rgb_to_hsl(1.0, 0.0, 0.0);
        assert!((h - 0.0).abs() < 1e-3);
        assert!((s - 1.0).abs() < 1e-3);
        assert!((l - 0.5).abs() < 1e-3);

        let (r, g, b) = hsl_to_rgb(h, s, l);
        assert!((r - 1.0).abs() < 1e-3);
        assert!(g.abs() < 1e-3);
        assert!(b.abs() < 1e-3);
    }

    #[test]
    fn test_matte_isolation() {
        let mut q = HslQualifier::default();
        q.hue_center = 0.0; // Red
        q.hue_width = 30.0;
        q.sat_center = 0.75;
        q.sat_width = 0.5;
        q.luma_center = 0.5;
        q.luma_width = 0.6;

        // Pure red
        let (h, s, l) = rgb_to_hsl(1.0, 0.0, 0.0);
        let factor = calculate_matte_factor(h, s, l, &q);
        assert_eq!(factor, 1.0);

        // Pure blue (Hue 240 deg)
        let (hb, sb, lb) = rgb_to_hsl(0.0, 0.0, 1.0);
        let factor_b = calculate_matte_factor(hb, sb, lb, &q);
        assert_eq!(factor_b, 0.0);
    }
}
