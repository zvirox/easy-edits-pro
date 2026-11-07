// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Automated shot matcher (AI-native plan Packet 3.3):
//! `color.match_shot_to_reference` — per-channel CDF alignment of a target
//! shot to a reference hero shot, with an optional skin-tone lock.
//!
//! Layering follows the established pattern (`speech.transcribe_to_subtitles`):
//! heavyweight media decoding happens in the caller (CLI / bridge), which
//! resolves the two clips, decodes one frame at each clip's midpoint through
//! [`crate::frame_extractor`], runs [`measurement`] + [`fit_match`], and hands
//! the engine the fitted result. The engine (saturn-core) stays decode-free
//! and owns project state: it validates the fit and applies it as a
//! primary-wheels node tagged `origin = "shot_match"` on the source clip.
//!
//! Domain end to end: decoded Rec.709-encoded display signal → scene-linear
//! ACEScg via the `rec709_video` IDT → statistics in ACEScct log domain
//! (per-channel CDF alignment) → fitted correction expressed in ACEScg linear
//! as primary-wheels parameters (the only node kind with HDR headroom; curves
//! and 3D LUTs clamp at 1.0). Gains and exposure are exact multiplicatives
//! and gamma an exact exponent in the wheels evaluation order
//! `out = (v · 2^E · G_c)^(2^−Γ) + O_c`, so a parametric exposure /
//! white-balance / gamma mismatch is fitted exactly, not approximated.
//!
//! Documented design choices (§4.4): decoded frames are assumed Rec.709
//! OETF-encoded (per-asset IDT selection is a later checklist item). The
//! wheels gain split uses the neutral constraint «Σ ln G = 0» to separate
//! shared exposure from per-channel gain. The skin-tone lock anchors the
//! *hue angle* of skin-band pixels onto the reference shot's angle by a
//! bounded grid search over the R/B log-gain difference; luma stays governed
//! by the global CDF fit and the residual skin luma is reported, not
//! corrected.

use saturn_core::color::spaces::{WorkingSpace, rec709_from_xyz_d65};
use saturn_core::color::transforms::{M_AP0_TO_XYZ_D65, Mat3, mat3_apply, rec709_oetf_encode};
use saturn_core::project::{PrimaryWheelsPayload, Wheel};

/// Search range for the R/B log-gain difference during the skin lock, in
/// log2 units (±0.5 ≈ ±41% on each channel relative to the other).
const SKIN_LOCK_SEARCH: f64 = 0.5;
/// Grid steps for the lock search (a deterministic grid, no monotonicity
/// assumptions).
const SKIN_LOCK_STEPS: usize = 41;

/// The widest per-channel gain the wheels encoding can represent:
/// `1 + gain ∈ (0.1, 5.0)` maps to |ln G| < ln 3 around the neutrality split
/// point used by the fit (value is ln 3, a compile-time literal).
const MAX_LOG_GAIN_DEVIATION: f64 = 1.098_612_288_668_109_8;

/// Quantile pairs are only trusted where both linear values sit above the
/// 8-bit toe plateau: raw codes 0..~3 quantise any scene's darkest signal
/// into the same couple of ACEScct bins, so pairs at or below this floor are
/// sensor/code noise, not a measurable mapping (including them lets that
/// noise steer the least-squares slope). A frame that is mostly toe makes
/// fewer than the minimum pairs and fails as [`ShotMatchError::InsufficientRange`].
const FIT_FLOOR_LINEAR: f64 = 0.004;

/// Fitted primary-wheels correction mapping the target shot onto the
/// reference shot (all values in ACEScg scene-linear terms).
#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct ShotMatch {
    /// Global exposure offset in stops (`v *= 2^stops`).
    pub exposure_stops: f64,
    /// Per-channel multiplicative gain (R, G, B) from the neutrality split.
    pub gain: [f64; 3],
    /// Wheels gamma exponent parameter (`v^(2^−gamma)`).
    pub gamma: f64,
    /// Per-channel additive offset (R, G, B).
    pub offset: [f64; 3],
    /// Skin-tone lock diagnostics. `None` when the lock was not requested;
    /// `Some` with `applied: false` when requested but skin was not detected
    /// on both frames (reported, never silent).
    pub skin: Option<SkinLock>,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct SkinLock {
    /// Whether an R/B gain correction was applied.
    pub applied: bool,
    /// Skin-band pixel count on the target frame (detection basis).
    pub pixels: usize,
    /// Mean target skin chroma angle before grading (vectorscope degrees).
    /// `None` when that frame had no detectable skin band.
    pub source_hue_deg: Option<f64>,
    /// Mean reference skin chroma angle (the anchor).
    pub reference_hue_deg: Option<f64>,
    /// Mean target skin chroma angle after the full graded correction.
    pub graded_hue_deg: Option<f64>,
    /// Final distance between the graded target's mean band hue direction
    /// and the reference's (normalised magnitude-weighted unit vectors) —
    /// the quantity the lock search minimised. `None` where either frame
    /// had no detectable skin band.
    pub chroma_vector_err_codes: Option<f64>,
    /// Skin-band mean luma IRE: target → graded; the reference's own IRE is
    /// the anchor telemetry. Luma is governed by the global CDF fit, not by
    /// the lock — residuals are reported, not corrected. `None` where the
    /// corresponding frame had no detectable skin band.
    pub source_ire: Option<f64>,
    pub graded_ire: Option<f64>,
    pub reference_ire: Option<f64>,
}

#[derive(Debug, PartialEq)]
pub enum ShotMatchError {
    /// Frame buffer does not match the declared dimensions.
    FrameSize { width: u32, height: u32, got: usize },
    /// A quantile mapping the parametric wheels model cannot represent
    /// (non-monotone CDF match, or a correction outside the wheels ranges).
    NotRepresentable(String),
    /// The frames have too little dynamic range to align sensibly.
    InsufficientRange,
}

impl std::fmt::Display for ShotMatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FrameSize { width, height, got } => write!(
                f,
                "frame buffer mismatch: {width}x{height} needs {} bytes, got {got}",
                *width as usize * *height as usize * 3
            ),
            Self::NotRepresentable(msg) => {
                write!(f, "shot match: correction not representable: {msg}")
            }
            Self::InsufficientRange => {
                write!(
                    f,
                    "shot match: frames have too little dynamic range to align"
                )
            }
        }
    }
}
impl std::error::Error for ShotMatchError {}

// --- Parametric fit ----------------------------------------------------------

/// Joint per-channel fit of the monotone model `ln r = k · ln t + b_c`
/// over the quantile pairs both of whose ends sit above the 8-bit toe floor
/// ([`FIT_FLOOR_LINEAR`]). One slope is shared across channels and only the
/// intercept varies per channel — exactly the expressiveness of the
/// persisted primary-wheels form (one gamma, per-channel gain/exposure), so
/// per-channel slope noise cannot leak in as dark-end hue error. Estimator
/// per channel: a Theil–Sen median slope pick (immune to the leverage of
/// noisy dark-end pairs) for MAD-trimming, then pooled weighted least
/// squares over the union of inliers, each pair weighted by its own 8-bit
/// quantisation noise. Returns `(k, [b_r, b_g, b_b])`. Degenerate channels
/// are typed errors.
fn fit_channels_joint(
    mappings: &[([f64; QUANTILES], [f64; QUANTILES]); 3],
) -> Result<(f64, [f64; 3]), ShotMatchError> {
    let mut kept: [Vec<(f64, f64)>; 3] = [Vec::new(), Vec::new(), Vec::new()];
    for (ch, (t, r)) in mappings.iter().enumerate() {
        let mut pairs: Vec<(f64, f64)> = Vec::with_capacity(QUANTILES);
        for k in 0..QUANTILES {
            if t[k] > FIT_FLOOR_LINEAR && r[k] > FIT_FLOOR_LINEAR {
                pairs.push((t[k].ln(), r[k].ln()));
            }
        }
        if pairs.len() < 6 {
            return Err(ShotMatchError::InsufficientRange);
        }
        // Theil–Sen slope: median of pairwise gradients.
        let mut slopes: Vec<f64> = Vec::with_capacity(pairs.len() * pairs.len() / 2);
        for i in 0..pairs.len() {
            for j in (i + 1)..pairs.len() {
                let dx = pairs[j].0 - pairs[i].0;
                if dx.abs() > 1e-9 {
                    slopes.push((pairs[j].1 - pairs[i].1) / dx);
                }
            }
        }
        slopes.sort_by(|a, b| a.total_cmp(b));
        let k_ts = median(&slopes).ok_or(ShotMatchError::InsufficientRange)?;
        if !k_ts.is_finite() || !(0.05..=5.0).contains(&k_ts) {
            return Err(ShotMatchError::NotRepresentable(format!(
                "channel {ch} maps with slope {k_ts} outside the representable wheels domain"
            )));
        }
        // MAD-trim: keep pairs within 2.5 robust σ of the median residual.
        let mut residuals: Vec<f64> = pairs.iter().map(|(x, y)| y - k_ts * x).collect();
        residuals.sort_by(|a, b| a.total_cmp(b));
        let med = median(&residuals).unwrap_or(0.0);
        let mut abs_dev: Vec<f64> = residuals.iter().map(|e| (e - med).abs()).collect();
        abs_dev.sort_by(|a, b| a.total_cmp(b));
        let sigma = 1.482_6 * median(&abs_dev).unwrap_or(0.0);
        let inliers: Vec<(f64, f64)> = if sigma > 0.0 {
            pairs
                .iter()
                .zip(residuals.iter())
                .filter(|(_, e)| (**e - med).abs() <= 2.5 * sigma)
                .map(|(p, _)| *p)
                .collect()
        } else {
            pairs.clone()
        };
        if inliers.len() < 6 {
            return Err(ShotMatchError::InsufficientRange);
        }
        kept[ch] = inliers;
    }

    // Pooled weighted fit with a shared slope: per-channel weighted means,
    // then the slope from within-channel weighted deviations, so colour is
    // carried entirely by the intercepts.
    let mut num = 0.0f64;
    let mut den = 0.0f64;
    let mut means = [(0.0f64, 0.0f64); 3];
    for (ch, pairs) in kept.iter().enumerate() {
        let mut wsum = 0.0f64;
        let (mut wx, mut wy) = (0.0, 0.0);
        let mut weights = Vec::with_capacity(pairs.len());
        for (x, y) in pairs {
            let sigma = log_quantisation_sigma(x.exp())
                .max(log_quantisation_sigma(y.exp()))
                .max(1e-6);
            let w = 1.0 / (sigma * sigma);
            weights.push(w);
            wsum += w;
            wx += w * x;
            wy += w * y;
        }
        if !wsum.is_finite() || wsum <= 0.0 {
            return Err(ShotMatchError::InsufficientRange);
        }
        means[ch] = (wx / wsum, wy / wsum);
        for ((x, y), w) in pairs.iter().zip(&weights) {
            num += w * (x - means[ch].0) * (y - means[ch].1);
            den += w * (x - means[ch].0) * (x - means[ch].0);
        }
    }
    if !den.is_finite() || den <= 1e-12 {
        return Err(ShotMatchError::InsufficientRange);
    }
    let k = num / den;
    if !k.is_finite() || !(0.05..=5.0).contains(&k) {
        return Err(ShotMatchError::NotRepresentable(format!(
            "pooled channel slope {k} outside the representable wheels domain"
        )));
    }
    let intercepts = [
        means[0].1 - k * means[0].0,
        means[1].1 - k * means[1].0,
        means[2].1 - k * means[2].0,
    ];
    Ok((k, intercepts))
}

/// Exact 709-linear ← ACEScg matrix: the inverse of the measurement decode
/// path (`rec709_to_ap0_matrix` then AP0→AP1), built once.
fn lin709_from_acescg_matrix() -> Mat3 {
    use saturn_core::color::spaces::rec709_to_ap0_matrix;
    use saturn_core::color::transforms::{M_AP1_TO_AP0, mat3_inv, mat3_mul};
    static M: std::sync::OnceLock<Mat3> = std::sync::OnceLock::new();
    *M.get_or_init(|| {
        mat3_mul(
            &mat3_inv(&rec709_to_ap0_matrix()).expect("709→AP0 invertible"),
            &M_AP1_TO_AP0,
        )
    })
}

/// Standard deviation of the log of an 8-bit-quantised, OETF-decoded value:
/// one code step (1/255 of signal) maps through the OETF slope into a
/// relative luminance error; ±½ code uniform → σ_code = 1/√12.

fn log_quantisation_sigma(acescg_value: f64) -> f64 {
    use saturn_core::color::transforms::rec709_oetf_encode;
    // ACEScg → 709-linear through the decode chain's exact inverse.
    let ap0 = WorkingSpace::AcesCg.to_ap0([acescg_value; 3]);
    let lin = mat3_apply(&lin709_from_acescg_matrix(), ap0);
    let cv = rec709_oetf_encode(lin[0].clamp(0.0, 1.0));
    // L = ((cv + 0.099)/1.099)^2.4 above the toe; linear below.
    let dln_l_dcode = if cv > 0.081 {
        2.4 / (cv + 0.099) / 255.0
    } else {
        1.0 / 255.0 / cv.max(1e-4)
    };
    (1.0 / 12.0f64.sqrt()) * dln_l_dcode
}

/// Median of a pre-sorted slice.
fn median(sorted: &[f64]) -> Option<f64> {
    if sorted.is_empty() {
        return None;
    }
    let mid = sorted.len() / 2;
    Some(if sorted.len() % 2 == 1 {
        sorted[mid]
    } else {
        (sorted[mid - 1] + sorted[mid]) / 2.0
    })
}

/// Applies the fitted (and optionally lock-adjusted) correction to one
/// ACEScg-linear sample: `out = (v · 2^E · G_c · 2^(±δ))^(2^−Γ) + O_c`.
/// The lock δ adds to ln G on red and subtracts on blue (a pure chroma-axis
/// correction around the skin line).
fn apply_fit(
    c: [f64; 3],
    exposure_stops: f64,
    gain: [f64; 3],
    gamma: f64,
    offset: [f64; 3],
    rb_delta: f64,
) -> [f64; 3] {
    let k = (-gamma).exp2(); // 2^−Γ
    let e2 = exposure_stops.exp2();
    let mut out = [0.0; 3];
    for ch in 0..3 {
        let rb = match ch {
            0 => rb_delta,
            2 => -rb_delta,
            _ => 0.0,
        };
        let g = gain[ch] * rb.exp2();
        out[ch] = (c[ch].max(0.0) * e2 * g).powf(k) + offset[ch];
    }
    out
}

/// ACEScg-linear → Rec.709-encoded signal (the decode's exact inverse
/// path: AP0 → XYZ D65 → Rec.709 → OETF), then the vectorscope angle, the
/// Rec.709-luma IRE, and the Cb/Cr chroma magnitude of that signal pixel.
fn signal_angle_ire(acescg: [f64; 3]) -> (f64, f64, f64) {
    let ap0 = WorkingSpace::AcesCg.to_ap0(acescg);
    let xyz = mat3_apply(&M_AP0_TO_XYZ_D65, ap0);
    let signal = mat3_apply(&rec709_from_xyz_d65(), xyz).map(rec709_oetf_encode);
    let signal = signal.map(|v| v.clamp(0.0, 1.0));
    let (r, g, b) = (signal[0], signal[1], signal[2]);
    let cb = 128.0 - 0.168_736 * r * 255.0 - 0.331_264 * g * 255.0 + 0.5 * b * 255.0;
    let cr = 128.0 + 0.5 * r * 255.0 - 0.418_688 * g * 255.0 - 0.081_312 * b * 255.0;
    let (u, v) = (cb - 128.0, cr - 128.0);
    let magnitude = u.hypot(v);
    let angle = if magnitude > 1.0 {
        (v / magnitude)
            .atan2(u / magnitude)
            .to_degrees()
            .rem_euclid(360.0)
    } else {
        0.0
    };
    let ire = (0.212_6 * r + 0.715_2 * g + 0.072_2 * b) * 100.0;
    (angle, ire, magnitude)
}

/// Perceptual polish of the fitted wheels parameters: deterministic
/// coordinate refinement of `(E, G, Γ, o)` minimising the σ-weighted
/// squared CIEDE2000 between the graded target's quantile ranks and the
/// reference's (both encoded through the continuous Rec.709 signal — no
/// code rounding). Returns the refined parameters with the neutral gain
/// convention restored (Σ ln G = 0).
fn polish_wheels(
    exposure_stops: f64,
    gain: [f64; 3],
    gamma: f64,
    offset: [f64; 3],
    mappings: &[([f64; QUANTILES], [f64; QUANTILES]); 3],
) -> (f64, [f64; 3], f64, [f64; 3]) {
    // Continuous signal encoding of the working-space channel triple
    // (AP0 → XYZ D65 → Rec.709 → OETF, no clamp-to-code rounding).
    let signal_of = |acescg: [f64; 3]| -> [f64; 3] {
        let ap0 = WorkingSpace::AcesCg.to_ap0(acescg);
        let xyz = mat3_apply(&M_AP0_TO_XYZ_D65, ap0);
        mat3_apply(&rec709_from_xyz_d65(), xyz).map(|v| rec709_oetf_encode(v.clamp(0.0, 1.0)))
    };
    // Quantisation weights: the reliability of each pair, as in the
    // log-space fit.
    let mut weights = [[1.0f64; QUANTILES]; 3];
    for ch in 0..3 {
        for k in 0..QUANTILES {
            let (t_v, r_v) = (mappings[ch].0[k], mappings[ch].1[k]);
            let sigma = log_quantisation_sigma(t_v)
                .max(log_quantisation_sigma(r_v))
                .max(1e-6);
            weights[ch][k] = 1.0 / (sigma * sigma);
        }
    }
    // Reference Lab per rank (fixed side of the objective), evaluated on
    // rank-aligned pseudo-pixels (rank k of each channel — the ladder
    // content is neutral-dominant, so the cross-channel rank mix tracks
    // real pixels closely).
    let mut ref_lab = [[[0.0f64; 3]; QUANTILES]; 3];
    for k in 0..QUANTILES {
        let pixel = [mappings[0].1[k], mappings[1].1[k], mappings[2].1[k]];
        let lab = saturn_perception::color_metrics::rgb_continuous_to_lab(
            signal_of(pixel)[0],
            signal_of(pixel)[1],
            signal_of(pixel)[2],
        );
        for ch in 0..3 {
            ref_lab[ch][k] = lab;
        }
    }
    let objective = |e: f64, g: &[f64; 3], ga: f64, o: &[f64; 3]| -> f64 {
        let p = (-ga).exp2();
        let e2 = e.exp2();
        let mut weighted = 0.0;
        for ch in 0..3 {
            for k in 0..QUANTILES {
                let graded_ch = (mappings[ch].0[k].max(0.0) * e2 * g[ch]).powf(p) + o[ch];
                // Rank-aligned pseudo-pixel with the graded channel in its
                // own slot and the reference ranks elsewhere.
                let mut pixel = [mappings[0].1[k], mappings[1].1[k], mappings[2].1[k]];
                pixel[ch] = graded_ch;
                let lab = saturn_perception::color_metrics::rgb_continuous_to_lab(
                    signal_of(pixel)[0],
                    signal_of(pixel)[1],
                    signal_of(pixel)[2],
                );
                let d = saturn_perception::color_metrics::ciede2000(lab, ref_lab[ch][k]);
                weighted += weights[ch][k] * d * d;
            }
        }
        weighted
    };

    // θ: exposure, ln gains ×3, gamma, offsets ×3. Coordinate refinement
    // with halving steps; ranges mirror the wheels revalidation in
    // `fit_match`.
    let mut e = exposure_stops;
    let mut lng = [gain[0].ln(), gain[1].ln(), gain[2].ln()];
    let mut ga = gamma;
    let mut o = offset;
    let mut steps = [0.04f64, 0.004, 0.004, 0.004, 0.01, 4.0e-4, 4.0e-4, 4.0e-4];
    let mut best = objective(e, &gain, ga, &o);
    for _sweep in 0..7 {
        for i in 0..8 {
            for dir in [-1.0f64, 1.0] {
                let (trial_e, trial_lng, trial_ga, trial_o, trial_best) = {
                    let mut te = e;
                    let mut tg = lng;
                    let mut tga = ga;
                    let mut to = o;
                    match i {
                        0 => te += dir * steps[0],
                        1..=3 => tg[i - 1] += dir * steps[1],
                        4 => tga += dir * steps[4],
                        _ => to[i - 5] += dir * steps[5],
                    }
                    // Range guards (wheels representability).
                    let gains_ok = tg.iter().all(|v| v.abs() <= MAX_LOG_GAIN_DEVIATION);
                    let ok = gains_ok
                        && te.abs() <= 10.0
                        && tga.abs() <= 4.0
                        && to.iter().all(|v| v.abs() <= 1.0)
                        && tg.iter().all(|v| v.is_finite());
                    if ok {
                        let g_arr = [tg[0].exp(), tg[1].exp(), tg[2].exp()];
                        let v = objective(te, &g_arr, tga, &to);
                        (te, tg, tga, to, v)
                    } else {
                        (e, lng, ga, o, f64::INFINITY)
                    }
                };
                if trial_best < best {
                    best = trial_best;
                    e = trial_e;
                    lng = trial_lng;
                    ga = trial_ga;
                    o = trial_o;
                }
            }
        }
        for s in &mut steps {
            *s *= 0.5;
        }
    }
    // Restore the neutral gain convention: (E → E + mean(lnG)/ln2, G →
    // G·2^−mean(lnG)) leaves every graded value invariant.
    let shift = (lng[0] + lng[1] + lng[2]) / 3.0;
    e += shift / std::f64::consts::LN_2;
    for v in &mut lng {
        *v -= shift;
    }
    (e, [lng[0].exp(), lng[1].exp(), lng[2].exp()], ga, o)
}

/// Grows the correction into a primary-wheels payload (the persisted node
/// form the engine command applies).
impl ShotMatch {
    pub fn wheels_payload(&self) -> PrimaryWheelsPayload {
        let wheel3 = |values: [f64; 3]| Wheel {
            red: values[0],
            green: values[1],
            blue: values[2],
            luma: 0.0,
        };
        PrimaryWheelsPayload {
            gain: wheel3([self.gain[0] - 1.0, self.gain[1] - 1.0, self.gain[2] - 1.0]),
            offset: wheel3(self.offset),
            lift: Wheel::default(),
            gamma: self.gamma,
            exposure_stops: self.exposure_stops,
        }
    }
}

/// Fits the target → reference correction: per-channel log-space
/// least-squares over the CDF quantile mapping, then a neutral split of the
/// shared intercept into exposure and per-channel gains, plus the skin-tone
/// lock when requested.
pub fn fit_match(
    target: &FrameMeasurement,
    reference: &FrameMeasurement,
    lock_skin_tones: bool,
) -> Result<ShotMatch, ShotMatchError> {
    let mappings = target.channel_mappings(reference);

    // Joint shared-slope fit: one exponent (the wheels gamma), per-channel
    // intercepts split into exposure (shared) and gains (neutral sum).
    let (k_mean, intercepts) = fit_channels_joint(&mappings)?;
    let gamma = -k_mean.log2();
    if !(-4.0..=4.0).contains(&gamma) {
        return Err(ShotMatchError::NotRepresentable(format!(
            "solved gamma {gamma} outside [-4, 4]"
        )));
    }
    let exposure_stops =
        (intercepts[0] + intercepts[1] + intercepts[2]) / (3.0 * k_mean * std::f64::consts::LN_2);
    if !(-10.0..=10.0).contains(&exposure_stops) {
        return Err(ShotMatchError::NotRepresentable(format!(
            "solved exposure {exposure_stops} stops outside [-10, 10]"
        )));
    }
    let mut gain = [0.0f64; 3];
    for ch in 0..3 {
        let ln_g = intercepts[ch] / k_mean - exposure_stops * std::f64::consts::LN_2;
        if !ln_g.is_finite() || ln_g.abs() > MAX_LOG_GAIN_DEVIATION {
            return Err(ShotMatchError::NotRepresentable(format!(
                "channel {ch} gain e^{ln_g} outside the wheels gain range"
            )));
        }
        gain[ch] = ln_g.exp();
    }

    // Offset: the model's additive ACEScg term, estimated as a
    // quantisation-weighted mean of the per-pair additive residuals
    // `r − (t·A)^k`. The single lowest trusted pair would anchor the offset
    // on the noisiest point of the signal (±half a code near the toe is
    // ~±10% in linear there); averaging all trusted pairs with inverse-
    // variance weights shrinks that error by √n and recovers a true-zero
    // black shift as zero.
    let mut offset = [0.0f64; 3];
    for ch in 0..3 {
        let mut wsum = 0.0f64;
        let mut osum = 0.0f64;
        for (t_v, r_v) in mappings[ch].0.iter().zip(mappings[ch].1.iter()) {
            if *t_v > FIT_FLOOR_LINEAR && *r_v > FIT_FLOOR_LINEAR {
                let predicted = (t_v * exposure_stops.exp2() * gain[ch]).powf(k_mean);
                let o = r_v - predicted;
                // Output-domain noise ≈ r · σ_ln(r) (relative code noise).
                let sigma = (r_v * log_quantisation_sigma(*r_v)).max(1e-9);
                let w = 1.0 / (sigma * sigma);
                wsum += w;
                osum += w * o;
            }
        }
        let o = if wsum > 0.0 { osum / wsum } else { 0.0 };
        if !(-1.0..=1.0).contains(&o) {
            return Err(ShotMatchError::NotRepresentable(format!(
                "channel {ch} offset {o} outside [-1, 1]"
            )));
        }
        offset[ch] = o;
    }

    // Perceptual polish: refine the wheels parameters directly against the
    // CIEDE2000 objective over the matched quantile pairs (continuous
    // signal domain, no code rounding). The log-space fit is unbiased in
    // its own metric but its per-pair quantisation noise leaves a few
    // tenths of a percent of per-channel bias that ΔE punishes hardest in
    // the near-neutral range; a deterministic coordinate refinement on the
    // actual gate metric recovers it.
    let (exposure_stops, mut gain, gamma, offset) =
        polish_wheels(exposure_stops, gain, gamma, offset, &mappings);

    let mut skin = None;
    if lock_skin_tones {
        skin = Some(match (&target.skin, &reference.skin) {
            (Some(src), Some(dst)) => {
                // Grid-search the R/B log-gain difference so the graded skin
                // chroma *vector* (angle and magnitude, in code units) lands
                // on the reference's; report the hue angles and luma
                // residual alongside (luma is never corrected by the lock).
                // An angle-only objective would be a trap: the vectorscope
                // hue rotates under an R/B gain tilt far less than the tilt
                // changes the actual colour (ΔE), so it would happily spend
                // a ten-percent gain change on a quarter-degree hue gain.
                let mut best = 0.0f64;
                let mut best_err = f64::INFINITY;
                for step in 0..SKIN_LOCK_STEPS {
                    let delta = -SKIN_LOCK_SEARCH
                        + 2.0 * SKIN_LOCK_SEARCH * step as f64 / (SKIN_LOCK_STEPS - 1) as f64;
                    let (ug, vg, _) =
                        graded_skin_stats(&src.samples, exposure_stops, gain, gamma, offset, delta);
                    let err = ((ug - dst.chroma_u).powi(2) + (vg - dst.chroma_v).powi(2)).sqrt();
                    if err < best_err {
                        best_err = err;
                        best = delta;
                    }
                }
                // Fold the winning chroma delta into the per-channel gains so
                // the persisted payload carries the full correction (a delta
                // field of its own would not exist in the wheels encoding).
                gain[0] *= best.exp2();
                gain[2] *= (-best).exp2();
                if gain.iter().any(|g| !g.is_finite() || *g <= 0.0 || *g > 5.0) {
                    return Err(ShotMatchError::NotRepresentable(
                        "skin-tone lock pushed a channel gain outside the wheels gain range"
                            .to_string(),
                    ));
                }
                let graded =
                    graded_skin_stats(&src.samples, exposure_stops, gain, gamma, offset, 0.0);
                SkinLock {
                    applied: best != 0.0,
                    pixels: src.pixels,
                    source_hue_deg: Some(src.angle_deg),
                    reference_hue_deg: Some(dst.angle_deg),
                    graded_hue_deg: Some((graded.1).atan2(graded.0).to_degrees().rem_euclid(360.0)),
                    chroma_vector_err_codes: Some(best_err),
                    source_ire: Some(src.ire_mean),
                    graded_ire: Some(graded.2),
                    reference_ire: Some(dst.ire_mean),
                }
            }
            (Some(src), None) => SkinLock {
                applied: false,
                pixels: src.pixels,
                source_hue_deg: Some(src.angle_deg),
                reference_hue_deg: None,
                graded_hue_deg: None,
                chroma_vector_err_codes: None,
                source_ire: Some(src.ire_mean),
                graded_ire: None,
                reference_ire: None,
            },
            (None, Some(dst)) => SkinLock {
                applied: false,
                pixels: 0,
                source_hue_deg: None,
                reference_hue_deg: Some(dst.angle_deg),
                graded_hue_deg: None,
                chroma_vector_err_codes: None,
                source_ire: None,
                graded_ire: None,
                reference_ire: Some(dst.ire_mean),
            },
            (None, None) => SkinLock {
                applied: false,
                pixels: 0,
                source_hue_deg: None,
                reference_hue_deg: None,
                graded_hue_deg: None,
                chroma_vector_err_codes: None,
                source_ire: None,
                graded_ire: None,
                reference_ire: None,
            },
        });
    }

    Ok(ShotMatch {
        exposure_stops,
        gain,
        gamma,
        offset,
        skin,
    })
}

/// Mean graded hue angle and luma IRE over the target's skin samples with
/// the given R/B lock delta.
fn graded_skin_stats(
    samples: &[[f64; 3]],
    exposure_stops: f64,
    gain: [f64; 3],
    gamma: f64,
    offset: [f64; 3],
    rb_delta: f64,
) -> (f64, f64, f64) {
    let mut sin_sum = 0.0;
    let mut cos_sum = 0.0;
    let mut ire_sum = 0.0;
    let mut mag_sum = 0.0;
    for sample in samples {
        let graded = apply_fit(*sample, exposure_stops, gain, gamma, offset, rb_delta);
        let (angle, ire, magnitude) = signal_angle_ire(graded);
        // Vectorscope convention: weight each pixel by its chroma
        // magnitude and normalise by the magnitude sum — near-neutral band
        // leaks then contribute proportionally to what they actually are,
        // instead of diluting the mean by their pixel count.
        let rad = angle.to_radians();
        sin_sum += rad.sin() * magnitude;
        cos_sum += rad.cos() * magnitude;
        ire_sum += ire * magnitude;
        mag_sum += magnitude;
    }
    if mag_sum <= 0.0 {
        return (f64::NAN, f64::NAN, f64::NAN);
    }
    (cos_sum / mag_sum, sin_sum / mag_sum, ire_sum / mag_sum)
}

// --- Measurement (split by area to honor the 800-line rule) ------------------
#[path = "shot_match_measurement.rs"]
mod measurement;

use self::measurement::QUANTILES;
pub use self::measurement::{
    FrameMeasurement, QUANTILE_SAMPLE_CAP, SKIN_LOCK_MIN_PIXELS, SKIN_SAMPLE_CAP, SkinStats,
    measurement,
};

// --- Tests -------------------------------------------------------------------

#[cfg(test)]
#[path = "shot_match_tests.rs"]
mod tests;
