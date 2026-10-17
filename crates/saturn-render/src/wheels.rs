use saturn_core::project::{ColorWheelParam, ColorWheels};

/// Applies Lift, Gamma, Gain, and Offset color wheel grading to an RGB triple.
/// Values are in [0.0, 1.0] range.
pub fn apply_color_wheels_rgb(r: f64, g: f64, b: f64, wheels: &ColorWheels) -> (f64, f64, f64) {
    let process_channel = |val: f64, lift: &ColorWheelParam, gamma: &ColorWheelParam, gain: &ColorWheelParam, offset: &ColorWheelParam, c_lift: f64, c_gamma: f64, c_gain: f64, c_offset: f64| -> f64 {
        let v = val.clamp(0.0, 1.0);

        // 1. Lift (affects shadows predominantly)
        let lift_val = (c_lift + lift.luma) * 0.5;
        let v_lift = v + lift_val * (1.0 - v);
        let v_lift = v_lift.clamp(0.0, 2.0);

        // 2. Gain (affects highlights predominantly)
        let gain_scale = (1.0 + c_gain + gain.luma).max(0.0);
        let v_gain = v_lift * gain_scale;

        // 3. Gamma (affects midtones)
        let gamma_exponent = (1.0 - (c_gamma + gamma.luma) * 0.5).clamp(0.1, 3.0);
        let v_gamma = if v_gain > 0.0 {
            v_gain.powf(gamma_exponent)
        } else {
            0.0
        };

        // 4. Offset (global pedestal shift)
        let offset_val = (c_offset + offset.luma) * 0.25;
        let v_out = v_gamma + offset_val;

        v_out.clamp(0.0, 1.0)
    };

    let r_out = process_channel(
        r,
        &wheels.lift,
        &wheels.gamma,
        &wheels.gain,
        &wheels.offset,
        wheels.lift.red,
        wheels.gamma.red,
        wheels.gain.red,
        wheels.offset.red,
    );

    let g_out = process_channel(
        g,
        &wheels.lift,
        &wheels.gamma,
        &wheels.gain,
        &wheels.offset,
        wheels.lift.green,
        wheels.gamma.green,
        wheels.gain.green,
        wheels.offset.green,
    );

    let b_out = process_channel(
        b,
        &wheels.lift,
        &wheels.gamma,
        &wheels.gain,
        &wheels.offset,
        wheels.lift.blue,
        wheels.gamma.blue,
        wheels.gain.blue,
        wheels.offset.blue,
    );

    (r_out, g_out, b_out)
}

/// Applies Lift, Gamma, Gain, and Offset in-place to an 8-bit RGBA image buffer.
pub fn apply_color_wheels_rgba(buffer: &mut [u8], wheels: &ColorWheels) {
    // Quick early-out if wheels are all neutral
    if wheels.lift == ColorWheelParam::default()
        && wheels.gamma == ColorWheelParam::default()
        && wheels.gain == ColorWheelParam::default()
        && wheels.offset == ColorWheelParam::default()
    {
        return;
    }

    for chunk in buffer.chunks_exact_mut(4) {
        let r = chunk[0] as f64 / 255.0;
        let g = chunk[1] as f64 / 255.0;
        let b = chunk[2] as f64 / 255.0;

        let (r_out, g_out, b_out) = apply_color_wheels_rgb(r, g, b, wheels);

        chunk[0] = (r_out * 255.0).round().clamp(0.0, 255.0) as u8;
        chunk[1] = (g_out * 255.0).round().clamp(0.0, 255.0) as u8;
        chunk[2] = (b_out * 255.0).round().clamp(0.0, 255.0) as u8;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_neutral_wheels_preserves_colors() {
        let wheels = ColorWheels::default();
        let (r, g, b) = apply_color_wheels_rgb(0.5, 0.5, 0.5, &wheels);
        assert!((r - 0.5).abs() < 1e-4);
        assert!((g - 0.5).abs() < 1e-4);
        assert!((b - 0.5).abs() < 1e-4);
    }

    #[test]
    fn test_lift_boosts_shadows_more_than_highlights() {
        let mut wheels = ColorWheels::default();
        wheels.lift.luma = 0.4;

        let (shadow_r, _, _) = apply_color_wheels_rgb(0.1, 0.1, 0.1, &wheels);
        let (highlight_r, _, _) = apply_color_wheels_rgb(0.9, 0.9, 0.9, &wheels);

        // Shadow gain delta should be larger than highlight gain delta
        let shadow_delta = shadow_r - 0.1;
        let highlight_delta = highlight_r - 0.9;
        assert!(shadow_delta > highlight_delta);
    }

    #[test]
    fn test_gain_boosts_highlights_more_than_shadows() {
        let mut wheels = ColorWheels::default();
        wheels.gain.luma = 0.5;

        let (shadow_r, _, _) = apply_color_wheels_rgb(0.1, 0.1, 0.1, &wheels);
        let (highlight_r, _, _) = apply_color_wheels_rgb(0.8, 0.8, 0.8, &wheels);

        let shadow_delta = shadow_r - 0.1;
        let highlight_delta = highlight_r - 0.8;
        assert!(highlight_delta > shadow_delta);
    }
}
