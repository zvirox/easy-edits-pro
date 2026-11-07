// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Color science helpers for perceptual metrics (Packet 2.2).
//!
//! Small, dependency-free CIE Lab conversion and the CIEDE2000 color
//! difference, shared by cut-continuity scoring and any future color match
//! reporting. Implementations are validated against the Sharma, Wu & Dalal
//! 2005 reference dataset (see the test at the bottom).

/// ITU-R BT.601 RGB (0..255) → CIE L\*a\*b\* (D65). The transfer curve is the
/// sRGB piecewise decode (not a pure gamma 2.2, and not Rec.709's OETF) —
/// fine for ΔE ranking, stated here so nobody mistakes it for a
/// colour-managed path.
pub fn rgb_to_lab(r: u8, g: u8, b: u8) -> [f64; 3] {
    rgb_continuous_to_lab(
        f64::from(r) / 255.0,
        f64::from(g) / 255.0,
        f64::from(b) / 255.0,
    )
}

/// Continuous-input twin of [`rgb_to_lab`]: RGB signal in 0..1 through the
/// same sRGB piecewise decode and D65 Lab matrices, without the 8-bit
/// rounding — so numeric optimisers can evaluate the metric smoothly.
pub fn rgb_continuous_to_lab(r: f64, g: f64, b: f64) -> [f64; 3] {
    let decode = |c: f64| -> f64 {
        if c <= 0.04045 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    };
    let [r, g, b] = [decode(r), decode(g), decode(b)];
    let x = 0.412_456_4 * r + 0.357_576_1 * g + 0.180_437_5 * b;
    let y = 0.212_672_9 * r + 0.715_152_2 * g + 0.072_175_0 * b;
    let z = 0.019_333_9 * r + 0.119_192_0 * g + 0.950_304_1 * b;
    // D65 white point.
    let (xn, yn, zn) = (0.950_47, 1.0, 1.088_83);
    let f = |t: f64| -> f64 {
        const EPS: f64 = 216.0 / 24_389.0;
        const KAPPA: f64 = 24_389.0 / 27.0;
        if t > EPS {
            t.cbrt()
        } else {
            (KAPPA * t + 16.0) / 116.0
        }
    };
    let (fx, fy, fz) = (f(x / xn), f(y / yn), f(z / zn));
    [116.0 * fy - 16.0, 500.0 * (fx - fy), 200.0 * (fy - fz)]
}

/// CIEDE2000 color difference (CIE 142; Sharma, Wu & Dalal 2005 reference
/// implementation, including the hue-chroma RT interaction term).
pub fn ciede2000(lab1: [f64; 3], lab2: [f64; 3]) -> f64 {
    let [l1, a1, b1] = lab1;
    let [l2, a2, b2] = lab2;
    let deg = |rad: f64| rad.to_degrees();
    let rad = |d: f64| d.to_radians();
    let c1 = (a1 * a1 + b1 * b1).sqrt();
    let c2 = (a2 * a2 + b2 * b2).sqrt();
    let c_bar = (c1 + c2) / 2.0;
    let c_bar7 = c_bar.powi(7);
    let g = 0.5 * (1.0 - (c_bar7 / (c_bar7 + 25.0_f64.powi(7))).sqrt());
    let a1p = (1.0 + g) * a1;
    let a2p = (1.0 + g) * a2;
    let c1p = (a1p * a1p + b1 * b1).sqrt();
    let c2p = (a2p * a2p + b2 * b2).sqrt();
    let h1p = if b1 == 0.0 && a1p == 0.0 {
        0.0
    } else {
        deg(b1.atan2(a1p)).rem_euclid(360.0)
    };
    let h2p = if b2 == 0.0 && a2p == 0.0 {
        0.0
    } else {
        deg(b2.atan2(a2p)).rem_euclid(360.0)
    };
    let dl = l2 - l1;
    let dc = c2p - c1p;
    let dhp = if c1p * c2p == 0.0 {
        0.0
    } else if (h2p - h1p).abs() <= 180.0 {
        h2p - h1p
    } else if h2p - h1p > 180.0 {
        h2p - h1p - 360.0
    } else {
        h2p - h1p + 360.0
    };
    let dh = 2.0 * (c1p * c2p).sqrt() * rad(dhp / 2.0).sin();
    let l_bar_p = (l1 + l2) / 2.0;
    let c_bar_p = (c1p + c2p) / 2.0;
    let h_bar_p = if c1p * c2p == 0.0 {
        h1p + h2p
    } else if (h1p - h2p).abs() <= 180.0 {
        (h1p + h2p) / 2.0
    } else if h1p + h2p < 360.0 {
        (h1p + h2p + 360.0) / 2.0
    } else {
        (h1p + h2p - 360.0) / 2.0
    };
    let t = 1.0 - 0.17 * rad(h_bar_p - 30.0).cos() + 0.24 * rad(2.0 * h_bar_p).cos()
        + 0.32 * rad(3.0 * h_bar_p + 6.0).cos() - 0.20 * rad(4.0 * h_bar_p - 63.0).cos();
    // Δθ is in degrees; RT = −2·√(C̄′⁷/(C̄′⁷+25⁷))·sin(2Δθ) per CIE 142.
    // These two terms were conflated in an early draft and produced NaN —
    // the reference pairs below pin the fix.
    let d_theta = 30.0 * (-((h_bar_p - 275.0) / 25.0).powi(2)).exp();
    let l_bar_p50sq = (l_bar_p - 50.0).powi(2);
    let sl = 1.0 + 0.015 * l_bar_p50sq / (20.0 + l_bar_p50sq).sqrt();
    let sc = 1.0 + 0.045 * c_bar_p;
    let sh = 1.0 + 0.015 * c_bar_p * t;
    let rt = -2.0 * (c_bar7 / (c_bar7 + 25.0_f64.powi(7))).sqrt() * rad(2.0 * d_theta).sin();
    let term_l = dl / (1.0 * sl);
    let term_c = dc / sc;
    let term_h = dh / sh;
    (term_l * term_l + term_c * term_c + term_h * term_h + rt * term_c * term_h).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ciede2000_matches_reference_pairs() {
        // Sharma, Wu & Dalal 2005 reference data.
        let cases: &[([f64; 3], [f64; 3], f64)] = &[
            ([50.0, 2.6772, -79.7751], [50.0, 0.0, -82.7485], 2.0425),
            ([50.0, 3.1571, -77.2803], [50.0, 0.0, -82.7485], 2.8615),
            ([50.0, 0.0, 0.0], [50.0, -1.0, 2.0], 2.3669),
            ([50.0, 2.49, -0.001], [50.0, -2.49, 0.0009], 7.1792),
            // Pairs exercising the RT (hue-chroma interaction) term.
            ([50.0, 2.5, -0.001], [50.0, 0.0, -2.5], 4.3065),
            ([50.0, 2.5, -0.001], [73.0, 25.0, -18.0], 27.1492),
            ([50.0, 2.5, -0.001], [61.0, -5.0, 29.0], 22.8977),
        ];
        for (i, (a, b, expected)) in cases.iter().enumerate() {
            let delta = ciede2000(*a, *b);
            assert!(
                (delta - expected).abs() < 0.02,
                "case {i}: got {delta}, expected {expected}"
            );
        }
    }

    #[test]
    fn identical_colors_have_zero_delta_e() {
        let lab = rgb_to_lab(120, 90, 40);
        assert!(ciede2000(lab, lab).abs() < 1e-9);
    }
}
