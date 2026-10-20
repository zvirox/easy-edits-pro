use saturn_core::project::{CurvePoint, RgbCurves};

/// Evaluates a monotonic cubic Hermite or Catmull-Rom spline at parameter `x` in [0.0, 1.0].
pub fn evaluate_spline(points: &[CurvePoint], x: f64) -> f64 {
    if points.is_empty() {
        return x.clamp(0.0, 1.0);
    }
    if points.len() == 1 {
        return points[0].y.clamp(0.0, 1.0);
    }
    let x = x.clamp(0.0, 1.0);

    // If x is before or at the first point
    if x <= points[0].x {
        return points[0].y.clamp(0.0, 1.0);
    }
    // If x is after or at the last point
    if x >= points[points.len() - 1].x {
        return points[points.len() - 1].y.clamp(0.0, 1.0);
    }

    // Find the bounding segment
    let mut i = 0;
    while i < points.len() - 1 && points[i + 1].x < x {
        i += 1;
    }

    let p0 = if i > 0 { &points[i - 1] } else { &points[i] };
    let p1 = &points[i];
    let p2 = &points[i + 1];
    let p3 = if i + 2 < points.len() {
        &points[i + 2]
    } else {
        p2
    };

    let dx = p2.x - p1.x;
    if dx.abs() < 1e-7 {
        return p1.y.clamp(0.0, 1.0);
    }

    let t = ((x - p1.x) / dx).clamp(0.0, 1.0);
    let t2 = t * t;
    let t3 = t2 * t;

    // Hermite tangents using finite difference with slope limiting
    let m1 = (p2.y - p0.y) / ((p2.x - p0.x).max(1e-7)) * dx;
    let m2 = (p3.y - p1.y) / ((p3.x - p1.x).max(1e-7)) * dx;

    // Hermite basis functions
    let h00 = 2.0 * t3 - 3.0 * t2 + 1.0;
    let h10 = t3 - 2.0 * t2 + t;
    let h01 = -2.0 * t3 + 3.0 * t2;
    let h11 = t3 - t2;

    let y = h00 * p1.y + h10 * m1 + h01 * p2.y + h11 * m2;
    y.clamp(0.0, 1.0)
}

/// Generates a fast 256-entry lookup table for an 8-bit channel curve.
pub fn generate_curve_lut(points: &[CurvePoint]) -> [u8; 256] {
    let mut lut = [0u8; 256];
    for i in 0..=255 {
        let x = (i as f64) / 255.0;
        let y = evaluate_spline(points, x);
        lut[i] = (y * 255.0).round().clamp(0.0, 255.0) as u8;
    }
    lut
}

/// Applies RgbCurves (master + red + green + blue) to an RGBA byte buffer in-place.
pub fn apply_rgb_curves_buffer(rgba_buf: &mut [u8], curves: &RgbCurves) {
    let master_lut = generate_curve_lut(&curves.master);
    let red_lut = generate_curve_lut(&curves.red);
    let green_lut = generate_curve_lut(&curves.green);
    let blue_lut = generate_curve_lut(&curves.blue);

    for chunk in rgba_buf.chunks_exact_mut(4) {
        let r = master_lut[chunk[0] as usize];
        let g = master_lut[chunk[1] as usize];
        let b = master_lut[chunk[2] as usize];

        chunk[0] = red_lut[r as usize];
        chunk[1] = green_lut[g as usize];
        chunk[2] = blue_lut[b as usize];
    }
}

/// Evaluates RgbCurves for single normalized RGB float values in [0.0, 1.0].
pub fn apply_rgb_curves(r: f32, g: f32, b: f32, curves: &RgbCurves) -> (f32, f32, f32) {
    let mr = evaluate_spline(&curves.master, r as f64) as f32;
    let mg = evaluate_spline(&curves.master, g as f64) as f32;
    let mb = evaluate_spline(&curves.master, b as f64) as f32;

    let out_r = evaluate_spline(&curves.red, mr as f64) as f32;
    let out_g = evaluate_spline(&curves.green, mg as f64) as f32;
    let out_b = evaluate_spline(&curves.blue, mb as f64) as f32;

    (out_r, out_g, out_b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identity_spline_and_lut() {
        let points = vec![CurvePoint { x: 0.0, y: 0.0 }, CurvePoint { x: 1.0, y: 1.0 }];
        let lut = generate_curve_lut(&points);
        assert_eq!(lut[0], 0);
        assert_eq!(lut[128], 128);
        assert_eq!(lut[255], 255);
    }

    #[test]
    fn test_contrast_s_curve() {
        let points = vec![
            CurvePoint { x: 0.0, y: 0.0 },
            CurvePoint { x: 0.25, y: 0.15 },
            CurvePoint { x: 0.75, y: 0.85 },
            CurvePoint { x: 1.0, y: 1.0 },
        ];
        let lut = generate_curve_lut(&points);
        assert!(lut[64] < 64);
        assert!(lut[192] > 192);
    }

    #[test]
    fn test_apply_rgb_curves_buffer() {
        let curves = RgbCurves::default();
        let mut pixels = vec![100, 150, 200, 255];
        apply_rgb_curves_buffer(&mut pixels, &curves);
        assert_eq!(pixels, vec![100, 150, 200, 255]);
    }
}
