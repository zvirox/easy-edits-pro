// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Frame measurement for the shot matcher: 8-bit Rec.709 signal decoded to
//! ACEScg via the `rec709_video` IDT, then the stride-sampled per-channel
//! quantile statistics and the vectorscope skin-band statistics the fit and
//! the skin lock consume. Split from [`super`] by area (measurement vs fit).

use super::{ShotMatchError, signal_angle_ire};
use saturn_core::color::spaces::{CameraIdt, WorkingSpace};
use saturn_core::color::transforms::acescct_encode;

/// Quantile grid for CDF alignment: 65 points at ~1.5% steps, centred away
/// from the unstable tail extremes. The pair noise at each quantile is the
/// ±half-code signal quantisation of both frames — ~1% in log-linear at
/// mid-tones — and independent quantiles (over a scene that resolves
/// distinct codes between them) average that down: intercept error scales
/// as σ_pair/√N. Grid points landing inside one code run repeat its value
/// and add nothing.
pub(crate) const QUANTILES: usize = 65;

/// ACEScct code-value binning for the CDF tables: 64 bins over [-0.4, 1.6].
/// The range covers the toe (encode(0) ≈ 0.073) through bright scene values;
/// values outside clamp into the end bins.
const CV_LO: f64 = -0.4;
const CV_HI: f64 = 1.6;
pub(crate) const CDF_BINS: usize = 64;

/// Skin-band detection below this pixel count reports "not detected" and the
/// lock reports no effect (never a silent success). Mirrors the scopes
/// telemetry's honesty about its detection basis (a share there; an absolute
/// count here because the lock acts on actual samples).
pub const SKIN_LOCK_MIN_PIXELS: usize = 64;

/// Cap on the stride-sampled skin colours kept per frame (the lock search
/// and diagnostics run on these, bounded to ~96 KB per frame).
pub const SKIN_SAMPLE_CAP: usize = 4096;

/// Measured skin-band statistics for one frame.
#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct SkinStats {
    /// Mean chroma angle in vectorscope degrees (0° = +U, CCW positive) over
    /// Rec.601 Cb/Cr — the same convention as the scopes telemetry.
    pub angle_deg: f64,
    /// Chroma-magnitude-weighted mean band hue direction, `(u, v)` — a
    /// normalised mean of unit vectors weighted by each sampled band
    /// pixel's chroma magnitude (each component in −1..1). The lock aligns
    /// this vector — angle *and* magnitude — because the hue angle alone
    /// barely responds to an R/B gain tilt (±0.5 stops of tilt rotates a
    /// typical skin hue well under 2°) and an angle-only objective would
    /// trade catastrophic gain changes for sub-degree hue wins. Luma is
    /// governed by the global CDF fit, not by the lock.
    pub chroma_u: f64,
    pub chroma_v: f64,
    /// Magnitude-weighted mean Rec.709-luma IRE over the sampled band
    /// pixels.
    pub ire_mean: f64,
    /// Band pixel count on the full frame (detection basis).
    pub pixels: usize,
    /// Stride-sampled ACEScg-linear colours of band pixels.
    pub samples: Vec<[f64; 3]>,
}

/// Sample cap for the per-channel quantile samples: at ~24 KB per channel
/// the quantile estimates stay well inside LSQ noise on real frames, and on
/// test-sized frames the stride collapses to 1 (every pixel sampled, exact).
pub const QUANTILE_SAMPLE_CAP: usize = 65536;

/// Per-frame measurement. The per-channel quantile source is a
/// deterministic, stride-sampled set of the frame's scene-linear ACEScg
/// channel values, sorted ascending — the exact pixel values (no histogram
/// grid bias; the stride keeps memory bounded on 4K frames). The ACEScct
/// luma CDF is coarse diagnostic telemetry (64 bins over
/// [`CV_LO`]..[`CV_HI`]).
#[derive(Clone, Debug)]
pub struct FrameMeasurement {
    /// Sorted ascending ACEScg channel samples — order statistics for the
    /// inverse marginal CDF (quantiles).
    pub channel_samples: [Vec<f64>; 3],
    /// Pixels represented by the samples (the stride denominator).
    pub sampled_pixels: usize,
    /// ACEScct-luma CDF (bins = [`CDF_BINS`] over [`CV_LO`]..[`CV_HI`]),
    /// ascending, normalised 0..1 — diagnostic telemetry.
    pub luma_cdf: Vec<f64>,
    pub skin: Option<SkinStats>,
}

fn cdf_bin_of(cv: f64) -> usize {
    (((cv - CV_LO) * CDF_BINS as f64 / (CV_HI - CV_LO)) as i64).clamp(0, (CDF_BINS - 1) as i64)
        as usize
}

/// Computes the per-frame measurement from decoded RGB8 (Rec.709-encoded
/// display signal).
pub fn measurement(
    width: u32,
    height: u32,
    rgb: &[u8],
) -> Result<FrameMeasurement, ShotMatchError> {
    let expected = width as usize * height as usize * 3;
    if width == 0 || height == 0 || rgb.len() < expected {
        return Err(ShotMatchError::FrameSize {
            width,
            height,
            got: rgb.len(),
        });
    }
    let pixels = width as usize * height as usize;

    let mut luma_counts = vec![0u64; CDF_BINS];
    let mut total = 0u64;
    let mut angle_sin_sum = 0.0f64;
    let mut angle_cos_sum = 0.0f64;
    let mut mag_sum = 0.0f64;
    let mut skin_ire_sum = 0.0f64;
    let mut skin_pixels = 0usize;
    // In-order strides keep both sample sets bounded without biasing toward
    // one image region.
    let skin_stride = (pixels / SKIN_SAMPLE_CAP).max(1);
    let quantile_stride = (pixels / QUANTILE_SAMPLE_CAP).max(1);
    let mut skin_samples: Vec<[f64; 3]> = Vec::with_capacity(SKIN_SAMPLE_CAP.min(pixels));
    let mut channel_samples: [Vec<f64>; 3] = [Vec::new(), Vec::new(), Vec::new()];
    for v in &mut channel_samples {
        v.reserve((pixels / quantile_stride).min(QUANTILE_SAMPLE_CAP));
    }

    for p in 0..pixels {
        let cv = [
            f64::from(rgb[p * 3]) / 255.0,
            f64::from(rgb[p * 3 + 1]) / 255.0,
            f64::from(rgb[p * 3 + 2]) / 255.0,
        ];
        // Rec.709-encoded signal → scene-linear ACEScg.
        let ap0 = CameraIdt::Rec709Video.decode_to_ap0(cv);
        let acescg = WorkingSpace::AcesCg.from_ap0(ap0);
        let luma = 0.212_6 * acescg[0] + 0.715_2 * acescg[1] + 0.072_2 * acescg[2];
        luma_counts[cdf_bin_of(acescct_encode(luma))] += 1;
        total += 1;
        if p % quantile_stride == 0 {
            for (ch, v) in acescg.iter().enumerate() {
                channel_samples[ch].push(*v);
            }
        }

        // Skin band: Rec.601 Cb/Cr (the vectorscope's convention), the same
        // broadcast band boxes the scopes telemetry uses.
        let cb = 128.0 - 0.168_736 * f64::from(rgb[p * 3]) - 0.331_264 * f64::from(rgb[p * 3 + 1])
            + 0.5 * f64::from(rgb[p * 3 + 2]);
        let cr = 128.0 + 0.5 * f64::from(rgb[p * 3])
            - 0.418_688 * f64::from(rgb[p * 3 + 1])
            - 0.081_312 * f64::from(rgb[p * 3 + 2]);
        let (u, v) = (cb - 128.0, cr - 128.0);
        let magnitude = u.hypot(v);
        if cb >= 77.0 && cb <= 127.0 && cr >= 133.0 && cr <= 173.0 && magnitude > 1.0 {
            skin_pixels += 1;
            if p % skin_stride == 0 && skin_samples.len() < SKIN_SAMPLE_CAP {
                skin_samples.push(acescg);
                // Band statistics accumulate on the sampled pixels' decoded,
                // continuously re-encoded signal — the same evaluation the
                // graded-side instrument (`graded_skin_stats`) performs —
                // and are weighted by chroma magnitude. Measuring the mean
                // on raw integer codes bakes each frame's round-half chroma
                // bias (observed ~0.4–1° at typical skin chroma) into the
                // band angle, which the skin lock would then chase as a
                // phantom hue error with an actual R/B tilt.
                let (angle, ire, mag) = signal_angle_ire(acescg);
                let rad = angle.to_radians();
                mag_sum += mag;
                angle_sin_sum += rad.sin() * mag;
                angle_cos_sum += rad.cos() * mag;
                skin_ire_sum += ire * mag;
            }
        }
    }

    let total_f = total as f64;
    for v in &mut channel_samples {
        v.sort_by(|a, b| a.total_cmp(b));
    }
    let mut luma_cdf = Vec::with_capacity(CDF_BINS);
    let mut acc = 0u64;
    for count in &luma_counts {
        acc += count;
        luma_cdf.push(acc as f64 / total_f);
    }

    let skin = (skin_pixels >= SKIN_LOCK_MIN_PIXELS).then(|| {
        // Statistics are magnitude-weighted means over the sampled band
        // pixels (those pushed to `samples`), on their decoded continuously
        // re-encoded signal: each pixel's weight is its own chroma
        // magnitude, so low-magnitude band leaks (bright neutrals that
        // round into the box on a colour-cast frame) cannot dilute the
        // mean — dividing by pixel count instead would scale the mean
        // vector by the band's incidental leak share, which differs
        // structurally between frames (the target's neutrals sit where the
        // shift put its colour cast; the reference's do not).
        let w = if mag_sum > 0.0 { mag_sum } else { 1.0 };
        let mean_u = angle_cos_sum / w;
        let mean_v = angle_sin_sum / w;
        SkinStats {
            angle_deg: mean_v.atan2(mean_u).to_degrees().rem_euclid(360.0),
            chroma_u: mean_u,
            chroma_v: mean_v,
            ire_mean: skin_ire_sum / w,
            pixels: skin_pixels,
            samples: skin_samples,
        }
    });

    Ok(FrameMeasurement {
        channel_samples,
        sampled_pixels: pixels,
        luma_cdf,
        skin,
    })
}

// --- CDF inversion and quantile mapping --------------------------------------

impl FrameMeasurement {
    /// Inverse marginal CDF for one channel: the ACEScg-linear value at
    /// quantile `q` (0..1) from the sorted sample order statistics (linear
    /// interpolation between adjacent order statistics).
    pub(crate) fn quantile(&self, ch: usize, q: f64) -> f64 {
        let samples = &self.channel_samples[ch];
        let n = samples.len();
        if n == 0 {
            return 0.0;
        }
        let q = q.clamp(0.0, 1.0);
        let pos = q * (n - 1) as f64;
        let lo = pos.floor() as usize;
        let hi = pos.ceil() as usize;
        if lo == hi {
            return samples[lo];
        }
        let frac = pos - lo as f64;
        samples[lo] * (1.0 - frac) + samples[hi] * frac
    }

    /// Per-channel quantile-pair tables: target-linear and reference-linear
    /// values at the same cumulative probability.
    pub(crate) fn channel_mappings(
        &self,
        reference: &Self,
    ) -> [([f64; QUANTILES], [f64; QUANTILES]); 3] {
        core::array::from_fn(|ch| {
            let mut t = [0.0; QUANTILES];
            let mut r = [0.0; QUANTILES];
            for (k, cell) in t.iter_mut().enumerate() {
                let q = (k as f64 + 0.5) / QUANTILES as f64;
                *cell = self.quantile(ch, q);
                r[k] = reference.quantile(ch, q);
            }
            (t, r)
        })
    }
}
