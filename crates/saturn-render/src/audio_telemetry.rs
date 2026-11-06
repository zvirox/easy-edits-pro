// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Audio telemetry for the AI feedback loop (Packet 1.3).
//!
//! Extends the EBU R128 measurement stack with the plan's audio report: a
//! 10-band octave spectrum, stereo phase correlation, and a
//! dialogue-to-background SNR estimate. As with the video scopes (Packet
//! 1.2) every number is a **direct measurement** of the decoded PCM —
//! there is deliberately no invented aggregate "mix quality" score
//! (plan-review §4.4), and the two warnings are labelled heuristics
//! derived from band energies.
//!
//! The FFT is a small in-crate iterative radix-2 Cooley–Tukey on
//! power-of-two windows: deterministic across machines (fixed float-op
//! order, no concurrency), no new dependency, and only ever fed
//! 2048 / 8192-sample windows.

use serde::{Deserialize, Serialize};
use saturn_core::Tick;

use crate::loudness::{EbuR128Meter, LoudnessStandard};

/// Octave band center frequencies (Hz), matching the plan's
/// `spectral_bands_db: [f32; 10]` layout: 31 Hz .. 16 kHz.
pub const BAND_CENTER_HZ: [f64; 10] = [
    31.0, 63.0, 125.0, 250.0, 500.0, 1_000.0, 2_000.0, 4_000.0, 8_000.0, 16_000.0,
];

/// Octave-band edge multiplier for band membership: a bin belongs to the
/// band whose center is the nearest power-of-two multiple within
/// `×/÷ 2^(1/6)` (one octave band split at its sixth-octave edges).
const OCTAVE_EDGE: f64 = 1.122_462_048_309_373; // 2^(1/6)

/// Speech/dialogue band for the SNR estimate (the classic dialogue-width
/// band). A heuristic band, not an ASR transcript.
const SPEECH_LOW_HZ: f64 = 300.0;
const SPEECH_HIGH_HZ: f64 = 3_400.0;
/// Sibilance zone examined for the 6 kHz warning.
const SIBILANCE_LOW_HZ: f64 = 4_700.0;
const SIBILANCE_HIGH_HZ: f64 = 10_000.0;
/// Separation (dB) the percentile split must exceed before a SNR is
/// reported; below this there is no distinguishable background.
const SNR_MIN_SEPARATION_DB: f64 = 3.0;
/// Warning threshold: a band this far above the vocal midrange is flagged.
const WARNING_EXCESS_DB: f64 = 6.0;

/// Upper limit on analyzed PCM (decode cost + report honesty): analysis
/// covers at most this many seconds per call; `truncated_to_cap` says so.
pub const MAX_ANALYSIS_SECONDS: f64 = 120.0;

/// Measurable audio telemetry for one PCM range.
///
/// Every field is a direct measurement of the analyzed audio. The two
/// warnings are deterministic band-energy heuristics (derivation in the
/// field docs); they never gate anything on their own.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioTelemetryReport {
    pub start_tick: Tick,
    pub end_tick: Tick,
    /// EBU R128 / BS.1770-4 loudness block (momentary, short-term,
    /// integrated, LRA, true peak) over the analyzed range.
    pub loudness: crate::loudness::LoudnessReport,
    /// Octave-band energies in dB re 1.0 mean-square amplitude
    /// (Parseval-normalized; tonal material within ±3 dB), bands per
    /// [`BAND_CENTER_HZ`].
    pub spectral_bands_db: [f32; 10],
    /// Stereo phase correlation, −1.0 (out-of-phase cancellation) ..
    /// +1.0 (mono coherent). `None` for mono input.
    pub phase_correlation: Option<f32>,
    /// Dialogue-to-background SNR estimate in the speech band: short
    /// windows of speech-band energy, split at the 25th/75th percentiles;
    /// active level = mean of the ≥ p75 energies, background = mean of the
    /// ≤ p25 energies. `None` when there is no distinguishable separation
    /// (constant tone, silence, continuous full-band material).
    pub dialogue_to_background_snr_db: Option<f32>,
    /// Heuristic: the 250 Hz octave exceeds the vocal midrange (1–2 kHz
    /// mean) by ≥ [`WARNING_EXCESS_DB`] — usually mud/boom. Fix decisions
    /// stay human.
    pub mud_warning_250hz: bool,
    /// Heuristic: the 4.7–10 kHz zone exceeds the 1–2 kHz vocal band by
    /// ≥ [`WARNING_EXCESS_DB`] — often sibilance on dialogue.
    pub sibilance_warning_6khz: bool,
    pub sample_rate: u32,
    pub channels: usize,
    /// Seconds of PCM actually analyzed (after any range/cap clamping).
    pub analyzed_seconds: f64,
    /// `true` when the range exceeded [`MAX_ANALYSIS_SECONDS`] and was cut
    /// short (stated, so confidence can be judged).
    pub truncated_to_cap: bool,
}

// ---------------------------------------------------------------------------
// FFT
// ---------------------------------------------------------------------------

/// In-place iterative radix-2 Cooley–Tukey FFT on interleaved (re, im)
/// pairs. `len` must be a power of two.
fn fft_inplace(data: &mut [(f64, f64)]) {
    let n = data.len();
    debug_assert!(n.is_power_of_two());
    // Bit-reversal permutation.
    let mut j = 0usize;
    for i in 0..n {
        if i < j {
            data.swap(i, j);
        }
        let mut m = n >> 1;
        while m >= 1 && j & m != 0 {
            j ^= m;
            m >>= 1;
        }
        j |= m;
    }
    // Butterflies: for each stage, w = e^{-2πi/len} advanced per twiddle
    // step by repeated complex multiplication (identical order every run).
    let mut len = 2;
    while len <= n {
        let step_angle = -2.0 * std::f64::consts::PI / len as f64;
        let (w_re, w_im) = (step_angle.cos(), step_angle.sin());
        let half = len / 2;
        let mut start = 0;
        while start < n {
            let mut tw_re = 1.0;
            let mut tw_im = 0.0;
            for k in 0..half {
                let (u_re, u_im) = data[start + k];
                let bv = data[start + k + half];
                let v_re = bv.0 * tw_re - bv.1 * tw_im;
                let v_im = bv.0 * tw_im + bv.1 * tw_re;
                data[start + k] = (u_re + v_re, u_im + v_im);
                data[start + k + half] = (u_re - v_re, u_im - v_im);
                let next_re = tw_re * w_re - tw_im * w_im;
                tw_im = tw_re * w_im + tw_im * w_re;
                tw_re = next_re;
            }
            start += len;
        }
        len <<= 1;
    }
}

/// Exact per-window Σw² for the Hann window of length `n`.
fn hann_energy(n: usize) -> f64 {
    let mut w_sq_sum = 0.0;
    for i in 0..n {
        let w = 0.5 - 0.5 * (2.0 * std::f64::consts::PI * i as f64 / n as f64).cos();
        w_sq_sum += w * w;
    }
    w_sq_sum
}

/// Hann-windowed single-sided power spectrum of `window` normalized so the
/// bins sum to `n·mean-square-amplitude` (Parseval with the window's exact
/// energy): `P_k = 2|X_k|²/Σw²` for `1 ≤ k < n/2`.
fn window_power_spectrum(window: &[f64], hann_sq_energy: f64) -> Vec<f64> {
    let n = window.len();
    let mut freqs: Vec<(f64, f64)> = Vec::with_capacity(n);
    for (i, &x) in window.iter().enumerate() {
        let w = 0.5 - 0.5 * (2.0 * std::f64::consts::PI * i as f64 / n as f64).cos();
        freqs.push((x * w, 0.0));
    }
    fft_inplace(&mut freqs);
    freqs[1..n / 2]
        .iter()
        .map(|(re, im)| (re * re + im * im) * 2.0 / hann_sq_energy)
        .collect()
}

/// A power spectrum averaged over evenly spaced windows across the clip
/// (deterministic stride — not first-N-only), bins still Parseval-normalized
/// to mean-square amplitude units: `Σ bins ≈ n_frames · mean-square`.
struct AveragedSpectrum {
    power: Vec<f64>,
    bin_hz: f64,
    window_frames: usize,
    window_count: usize,
}

fn averaged_spectrum(samples: &[f64], sample_rate: u32, window: usize) -> AveragedSpectrum {
    if samples.len() < window {
        // Zero-pad a single window (report covers very short clips honestly
        // through `analyzed_seconds`).
        let mut padded = samples.to_vec();
        padded.resize(window, 0.0);
        let energy = hann_energy(window);
        let power = window_power_spectrum(&padded, energy);
        return AveragedSpectrum {
            power,
            bin_hz: sample_rate as f64 / window as f64,
            window_frames: window,
            window_count: 1,
        };
    }
    let count = samples.len() / window;
    let hop = ((samples.len() - window) / count).max(1);
    let energy = hann_energy(window);
    let mut agg = vec![0.0f64; window / 2 - 1];
    let mut offset = 0usize;
    for _ in 0..count {
        if offset + window > samples.len() {
            break;
        }
        let spectrum = window_power_spectrum(&samples[offset..offset + window], energy);
        for (dst, src) in agg.iter_mut().zip(spectrum) {
            *dst += src;
        }
        offset += hop;
    }
    let power: Vec<f64> = agg.into_iter().map(|p| p / count as f64).collect();
    AveragedSpectrum {
        power,
        bin_hz: sample_rate as f64 / window as f64,
        window_frames: window,
        window_count: count,
    }
}

impl AveragedSpectrum {
    /// Sum of bin powers with `low_hz ≤ f < high_hz` (inclusive edges
    /// snapped to the bin grid), in mean-square-amplitude units (divided
    /// by the window length as per the Parseval normalization).
    fn energy_between(&self, low_hz: f64, high_hz: f64) -> f64 {
        let n_bins = self.power.len() as f64;
        let nyquist = self.bin_hz * n_bins;
        let high = high_hz.min(nyquist);
        let first = ((low_hz / self.bin_hz).ceil() as usize).max(1);
        let last = (((high / self.bin_hz).floor() as usize) as f64)
            .min(n_bins as f64 - 1.0) as usize;
        if first > last {
            return 0.0;
        }
        let spectrum_frames = if self.window_count > 0 {
            self.window_frames
        } else {
            self.window_frames
        };
        self.power[first..=last].iter().sum::<f64>() / spectrum_frames as f64
    }

    /// dB re 1.0 mean-square amplitude.
    fn energy_db(&self, energy: f64) -> f64 {
        10.0 * energy.max(1e-12).log10()
    }
}

// ---------------------------------------------------------------------------
// Report assembly
// ---------------------------------------------------------------------------

/// Analyze interleaved PCM (`samples.len() == frames × channels`) into a
/// full [`AudioTelemetryReport`]. Errors on degenerate input (typed, not
/// placeholders).
pub fn analyze_pcm_audio_telemetry(
    samples: &[f32],
    channels: usize,
    sample_rate: u32,
    standard: LoudnessStandard,
    start_tick: Tick,
    end_tick: Tick,
) -> Result<AudioTelemetryReport, String> {
    let channels = channels.max(1);
    if samples.is_empty() {
        return Err("Audio telemetry needs a non-empty PCM range".into());
    }
    if sample_rate == 0 {
        return Err("Audio telemetry needs a positive sample rate".into());
    }
    let frames = samples.len() / channels;
    if frames == 0 {
        return Err("Audio telemetry needs at least one complete frame".into());
    }
    let cap_frames = (MAX_ANALYSIS_SECONDS * sample_rate as f64) as usize;
    let truncated = frames > cap_frames;
    let frames = frames.min(cap_frames.max(1));
    let analyzed_seconds = frames as f64 / sample_rate as f64;
    let analyzed = &samples[..frames * channels];

    // Loudness + phase correlation in one meter pass.
    let mut meter = EbuR128Meter::new(sample_rate, channels, standard);
    meter.process_interleaved(analyzed);
    let loudness = meter.get_report();
    let phase_correlation = meter.phase_correlation().map(|c| c as f32);

    // Mean-channel mono fold for spectral / SNR analysis (an average
    // channel's content: identical-correlated stereo reads at its true
    // per-channel level, not an energy-scaled bus mix).
    let scale = 1.0 / channels as f64;
    let mono: Vec<f64> = analyzed
        .chunks_exact(channels)
        .map(|frame| frame.iter().map(|&s| s as f64 * scale).sum())
        .collect();

    let spectrum = averaged_spectrum(&mono, sample_rate, 8192);

    // Octave bands: energy membership within ×/÷ 2^(1/6) of each center.
    let mut spectral_bands_db = [0.0f32; 10];
    let mut band_energies = [0.0f64; 10];
    for (i, &center) in BAND_CENTER_HZ.iter().enumerate() {
        let energy =
            spectrum.energy_between(center / OCTAVE_EDGE, center * OCTAVE_EDGE);
        band_energies[i] = energy;
        spectral_bands_db[i] = spectrum.energy_db(energy) as f32;
    }

    // Vocal band reference for the warnings: 1–2 kHz mean energy.
    let vocal_energy = (band_energies[5] + band_energies[6]) / 2.0;
    let vocal_db = spectrum.energy_db(vocal_energy);
    let mud = spectral_bands_db[3] >= (vocal_db + WARNING_EXCESS_DB) as f32;
    let sibilance_energy = spectrum.energy_between(SIBILANCE_LOW_HZ, SIBILANCE_HIGH_HZ);
    let sibilance = spectrum.energy_db(sibilance_energy) >= vocal_db + WARNING_EXCESS_DB;

    let dialogue_to_background_snr_db = dialogue_snr(&mono, sample_rate).map(|snr| snr as f32);

    Ok(AudioTelemetryReport {
        start_tick,
        end_tick,
        loudness,
        spectral_bands_db,
        phase_correlation,
        dialogue_to_background_snr_db,
        mud_warning_250hz: mud,
        sibilance_warning_6khz: sibilance,
        sample_rate,
        channels,
        analyzed_seconds,
        truncated_to_cap: truncated,
    })
}

/// Dialogue-to-background SNR: split 2048-sample windows of speech-band
/// energy at the 25th/75th percentiles, then ratio the mean active level
/// over the mean background level. `None` without a clear separation.
fn dialogue_snr(mono: &[f64], sample_rate: u32) -> Option<f64> {
    let window = (sample_rate as f64 * 0.02).round() as usize; // ~20 ms
    let window = window.max(256).next_power_of_two();
    if mono.len() < window * 8 {
        return None; // too short to separate activity from background
    }
    let energy = hann_energy(window);
    let count = mono.len() / window;
    let mut speech_powers = Vec::with_capacity(count);
    for i in 0..count {
        let start = i * window;
        let spectrum = window_power_spectrum(&mono[start..start + window], energy);
        // speech-band energy in mean-square units (window length divides).
        let band: f64 = {
            let first = ((SPEECH_LOW_HZ / (sample_rate as f64 / window as f64)).ceil() as usize)
                .max(1);
            let last = ((SPEECH_HIGH_HZ / (sample_rate as f64 / window as f64)).floor() as usize)
                .min(spectrum.len() - 1);
            if first > last {
                0.0
            } else {
                spectrum[first..=last].iter().sum::<f64>() / window as f64
            }
        };
        speech_powers.push(band);
    }

    speech_powers.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let p25 = percentile(&speech_powers, 0.25);
    let p75 = percentile(&speech_powers, 0.75);
    if p25 <= 0.0 || p75 <= 0.0 {
        return None;
    }
    let separation_db = 10.0 * (p75 / p25).log10();
    if separation_db < SNR_MIN_SEPARATION_DB {
        return None;
    }

    // Mean active (≥ p75) vs mean background (≤ p25) energies.
    let mut active_sum = 0.0;
    let mut active_n = 0.0;
    let mut background_sum = 0.0;
    let mut background_n = 0.0;
    for &p in &speech_powers {
        if p >= p75 {
            active_sum += p;
            active_n += 1.0;
        }
        if p <= p25 {
            background_sum += p;
            background_n += 1.0;
        }
    }
    if active_n == 0.0 || background_n == 0.0 || background_sum <= 0.0 {
        return None;
    }
    let snr = 10.0 * ((active_sum / active_n) / (background_sum / background_n)).log10();
    Some(snr.clamp(0.0, 60.0))
}

fn percentile(sorted: &[f64], which: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let index = ((sorted.len() as f64 - 1.0) * which).round() as usize;
    sorted[index.min(sorted.len() - 1)]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::loudness::KWeightingFilter;

    fn stereo(samples: impl Iterator<Item = (f64, f64)>) -> Vec<f32> {
        samples
            .flat_map(|(l, r)| [l as f32, r as f32])
            .collect()
    }

    #[test]
    fn fft_finds_an_exactly_on_bin_sine() {
        // A sine on bin 8 dominates that bin (the returned spectrum is
        // single-sided starting at bin 1, so vec index = bin − 1).
        let n = 1024;
        let k = 8.0;
        let signal: Vec<f64> = (0..n)
            .map(|i| (2.0 * std::f64::consts::PI * k * i as f64 / n as f64).sin())
            .collect();
        let power = window_power_spectrum(&signal, hann_energy(n));
        let peak = power
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
            .unwrap();
        assert_eq!(peak.0 + 1, 8, "peak bin should be 8, got {}", peak.0 + 1);
    }

    // -- exit criterion ---------------------------------------------------

    #[test]
    fn reference_ebu_r128_signal_measures_minus_23_lufs_plus_minus_tenth() {
        // ITU reference: a 1 kHz stereo sine reads −23 LUFS *under the
        // K-weighting filter*; calibrate the amplitude through the real
        // filter's measured steady-state gain at 1 kHz so the exit
        // criterion (−23.0 ± 0.1 LUFS) tests the meter, not filter lore.
        let sample_rate = 48_000_u32;
        let mut probe = KWeightingFilter::new(sample_rate);
        let gain = probe.steady_state_gain_at(sample_rate, 1_000.0);
        let target = 10.0_f64.powf((-23.0 + 0.691) / 20.0); // A·g for −23 LUFS
        let amplitude = target / gain;

        let seconds = 3;
        let frames = sample_rate as usize * seconds;
        let samples = stereo((0..frames).map(|i| {
            let t = i as f64 / sample_rate as f64;
            let v = amplitude * (2.0 * std::f64::consts::PI * 1_000.0 * t).sin();
            (v, v)
        }));

        let report = analyze_pcm_audio_telemetry(
            &samples,
            2,
            sample_rate,
            LoudnessStandard::EbuR128,
            Tick(0),
            Tick(frames as i64 * saturn_core::time::TICKS_PER_SECOND as i64 / sample_rate as i64),
        )
        .expect("reference signal analyzes");
        assert!(
            (report.loudness.integrated_lufs - (-23.0)).abs() <= 0.1,
            "exit criterion: integrated {} LUFS not within ±0.1 of −23.0",
            report.loudness.integrated_lufs
        );
        assert!(report.loudness.is_compliant);
    }

    #[test]
    fn phase_correlation_tracks_identical_inverted_and_mono() {
        let sample_rate = 48_000_u32;
        let frames = sample_rate as usize;
        let amp = 0.2_f64;
        let identical = stereo((0..frames).map(|i| {
            let v = amp * (2.0 * std::f64::consts::PI * 440.0 * i as f64 / sample_rate as f64)
                .sin();
            (v, v)
        }));
        let report = analyze_pcm_audio_telemetry(
            &identical, 2, sample_rate, LoudnessStandard::EbuR128, Tick(0), Tick(1),
        )
        .unwrap();
        let corr = report.phase_correlation.expect("stereo reports correlation");
        assert!(corr > 0.99, "identical channels = mono coherent: {corr}");

        let inverted = stereo((0..frames).map(|i| {
            let v = amp * (2.0 * std::f64::consts::PI * 440.0 * i as f64 / sample_rate as f64)
                .sin();
            (v, -v)
        }));
        let report = analyze_pcm_audio_telemetry(
            &inverted, 2, sample_rate, LoudnessStandard::EbuR128, Tick(0), Tick(1),
        )
        .unwrap();
        let corr = report.phase_correlation.expect("stereo reports correlation");
        assert!(corr < -0.99, "inverted channels = out of phase: {corr}");

        let mono: Vec<f32> = (0..frames)
            .map(|i| {
                (amp
                    * (2.0 * std::f64::consts::PI * 440.0 * i as f64 / sample_rate as f64)
                        .sin()) as f32
            })
            .collect();
        let report = analyze_pcm_audio_telemetry(
            &mono, 1, sample_rate, LoudnessStandard::EbuR128, Tick(0), Tick(1),
        )
        .unwrap();
        assert_eq!(report.phase_correlation, None, "mono has no phase metric");
    }

    #[test]
    fn octave_bands_localize_a_1khz_sine() {
        let sample_rate = 48_000_u32;
        let frames = sample_rate as usize; // 1 s
        let amp = 10.0_f64.powf(-20.0 / 20.0);
        let samples = stereo((0..frames).map(|i| {
            let v = amp * (2.0 * std::f64::consts::PI * 1_000.0 * i as f64 / sample_rate as f64)
                .sin();
            (v, v)
        }));
        let report = analyze_pcm_audio_telemetry(
            &samples, 2, sample_rate, LoudnessStandard::EbuR128, Tick(0), Tick(1),
        )
        .unwrap();
        let bands = &report.spectral_bands_db;
        let peak = bands
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
            .unwrap();
        assert_eq!(peak.0, 5, "1 kHz sine peaks in the 1 kHz band, got {} @ {}", peak.0, peak.1);
        // The energy preset: −20 dBFS amplitude → mean square A²/2 → ~−23 dB
        // (Hann leakage costs a couple of dB on a pure tone).
        assert!(
            (-29.0..=-21.0).contains(peak.1),
            "1 kHz band reads {} dB, expected ≈ −23 ± tolerance",
            peak.1
        );
        // Distant bands are far down (sub-bass and top octave).
        assert!(bands[1] < peak.1 - 40.0, "63 Hz = {} vs peak {}", bands[1], peak.1);
        assert!(bands[9] < peak.1 - 40.0, "16 kHz = {} vs peak {}", bands[9], peak.1);
        // No invented warnings on a clean sine.
        assert!(!report.mud_warning_250hz);
        assert!(!report.sibilance_warning_6khz);
    }

    #[test]
    fn dialogue_snr_separates_bursts_from_background_and_none_on_constant() {
        let sample_rate = 48_000_u32;
        // A page of "speech": 15 bursts (dialogue band tones) separated by
        // quiet broadband background.
        let mut samples: Vec<f32> = Vec::new();
        for _burst in 0..15 {
            for i in 0..sample_rate / 4 {
                let t = i as f64 / sample_rate as f64;
                let v = 0.3
                    * (0.6 * (2.0 * std::f64::consts::PI * 500.0 * t).sin()
                        + 0.4 * (2.0 * std::f64::consts::PI * 1_500.0 * t).sin());
                samples.push(v as f32);
                samples.push(v as f32);
            }
            for i in 0..sample_rate / 4 {
                let t = i as f64 / sample_rate as f64;
                // Low-level background inside the speech band (400 Hz).
                let v = 0.01 * (2.0 * std::f64::consts::PI * 400.0 * t).sin();
                samples.push(v as f32);
                samples.push(v as f32);
            }
        }
        let report = analyze_pcm_audio_telemetry(
            &samples, 2, sample_rate, LoudnessStandard::EbuR128, Tick(0), Tick(1),
        )
        .unwrap();
        let snr = report
            .dialogue_to_background_snr_db
            .expect("burst pattern has a distinguishable background");
        assert!(snr > 10.0, "bursts vs background should give high SNR: {snr}");

        // A constant tone has no separable background → None, not a number.
        let constant: Vec<f32> = (0..sample_rate * 2)
            .map(|i| {
                (0.2 * (2.0 * std::f64::consts::PI * 1_000.0 * i as f64 / sample_rate as f64)
                    .sin()) as f32
            })
            .collect();
        let report = analyze_pcm_audio_telemetry(
            &constant, 1, sample_rate, LoudnessStandard::EbuR128, Tick(0), Tick(1),
        )
        .unwrap();
        assert_eq!(report.dialogue_to_background_snr_db, None);
    }

    #[test]
    fn mud_and_sibilance_warnings_fire_on_their_bands() {
        let sample_rate = 48_000_u32;
        let frames = sample_rate as usize;
        // Loud 250 Hz boom over a quiet midrange → mud warning.
        let boom = stereo((0..frames).map(|i| {
            let v = 0.4 * (2.0 * std::f64::consts::PI * 250.0 * i as f64 / sample_rate as f64)
                .sin();
            (v, v)
        }));
        let report = analyze_pcm_audio_telemetry(
            &boom, 2, sample_rate, LoudnessStandard::EbuR128, Tick(0), Tick(1),
        )
        .unwrap();
        assert!(report.mud_warning_250hz, "250 Hz boom must trip the mud heuristic");

        // Energy in the sibilance zone (6 kHz) over a quiet midrange.
        let hiss = stereo((0..frames).map(|i| {
            let v = 0.4 * (2.0 * std::f64::consts::PI * 8_000.0 * i as f64 / sample_rate as f64)
                .sin();
            (v, v)
        }));
        let report = analyze_pcm_audio_telemetry(
            &hiss, 2, sample_rate, LoudnessStandard::EbuR128, Tick(0), Tick(1),
        )
        .unwrap();
        assert!(
            report.sibilance_warning_6khz,
            "8 kHz tone must trip the sibilance heuristic"
        );
    }

    #[test]
    fn degenerate_inputs_are_typed_errors() {
        assert!(analyze_pcm_audio_telemetry(
            &[],
            2,
            48_000,
            LoudnessStandard::EbuR128,
            Tick(0),
            Tick(1)
        )
        .is_err());
        assert!(analyze_pcm_audio_telemetry(
            &[0.0_f32; 100],
            2,
            0,
            LoudnessStandard::EbuR128,
            Tick(0),
            Tick(1)
        )
        .is_err());
    }
}
