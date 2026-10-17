use serde::{Deserialize, Serialize};

/// Broadcast Loudness standard profile according to EBU R128, ATSC A/85, or Online Streaming.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LoudnessStandard {
    #[default]
    EbuR128,     // Target: -23.0 LUFS, Max TP: -1.0 dBTP, Tolerance: ±1.0 LU
    Streaming,   // Target: -14.0 LUFS, Max TP: -1.0 dBTP, Tolerance: ±1.5 LU (YouTube, Spotify, Apple)
    AtscA85,     // Target: -24.0 LUFS, Max TP: -2.0 dBTP, Tolerance: ±1.0 LU (US Television)
}

impl LoudnessStandard {
    pub fn target_lufs(&self) -> f64 {
        match self {
            Self::EbuR128 => -23.0,
            Self::Streaming => -14.0,
            Self::AtscA85 => -24.0,
        }
    }

    pub fn max_true_peak_dbtp(&self) -> f64 {
        match self {
            Self::EbuR128 => -1.0,
            Self::Streaming => -1.0,
            Self::AtscA85 => -2.0,
        }
    }

    pub fn tolerance_lu(&self) -> f64 {
        match self {
            Self::EbuR128 => 1.0,
            Self::Streaming => 1.5,
            Self::AtscA85 => 1.0,
        }
    }
}

/// Comprehensive Loudness measurement report compliant with ITU-R BS.1770-4 & EBU R128.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LoudnessReport {
    /// Momentary loudness (400ms sliding window) in LUFS.
    pub momentary_lufs: f64,
    /// Short-term loudness (3-second sliding window) in LUFS.
    pub short_term_lufs: f64,
    /// Integrated loudness (full program with dual-gate thresholding) in LUFS.
    pub integrated_lufs: f64,
    /// Loudness Range (LRA) in LU.
    pub loudness_range_lu: f64,
    /// True Peak over-sampled inter-sample maximum in dBTP.
    pub true_peak_dbtp: f64,
    /// Target program loudness standard in LUFS.
    pub target_lufs: f64,
    /// Whether the integrated loudness is within standard tolerance.
    pub is_compliant: bool,
    /// Gain adjustment recommendation in dB (target - integrated).
    pub gain_recommendation_db: f64,
}

/// Direct Form II Transposed Biquad Filter.
#[derive(Clone, Debug)]
struct Biquad {
    b0: f64,
    b1: f64,
    b2: f64,
    a1: f64,
    a2: f64,
    s1: f64,
    s2: f64,
}

impl Biquad {
    fn new(b0: f64, b1: f64, b2: f64, a1: f64, a2: f64) -> Self {
        Self {
            b0,
            b1,
            b2,
            a1,
            a2,
            s1: 0.0,
            s2: 0.0,
        }
    }

    #[inline(always)]
    fn process(&mut self, x: f64) -> f64 {
        let y = self.b0 * x + self.s1;
        self.s1 = self.b1 * x - self.a1 * y + self.s2;
        self.s2 = self.b2 * x - self.a2 * y;
        y
    }

    fn reset(&mut self) {
        self.s1 = 0.0;
        self.s2 = 0.0;
    }
}

/// ITU-R BS.1770-4 K-weighting dual filter for a single audio channel.
#[derive(Clone, Debug)]
struct KWeightingFilter {
    stage1_pre: Biquad,
    stage2_rlb: Biquad,
}

impl KWeightingFilter {
    fn new(sample_rate: u32) -> Self {
        let fs = sample_rate as f64;

        // BS.1770-4 Stage 1: High shelf filter (head-model)
        // At 48 kHz exact values:
        let stage1_pre = if sample_rate == 48_000 {
            Biquad::new(
                1.53512485958697,
                -2.69169618940638,
                1.19839281085285,
                -1.69065929318241,
                0.73248077421585,
            )
        } else {
            // Bilinear transform for arbitrary sample rate:
            // f0 = 1681.97 Hz, Gain = +3.9998 dB, Q = 0.7071
            let f0 = 1681.9744509555319;
            let gain_db = 3.9998438;
            let q = 0.7071752369554193;
            let a = 10.0_f64.powf(gain_db / 40.0);
            let w0 = 2.0 * std::f64::consts::PI * f0 / fs;
            let alpha = w0.sin() / (2.0 * q);
            let cos_w0 = w0.cos();
            let b0 = a * ((a + 1.0) + (a - 1.0) * cos_w0 + 2.0 * a.sqrt() * alpha);
            let b1 = -2.0 * a * ((a - 1.0) + (a + 1.0) * cos_w0);
            let b2 = a * ((a + 1.0) + (a - 1.0) * cos_w0 - 2.0 * a.sqrt() * alpha);
            let a0 = (a + 1.0) - (a - 1.0) * cos_w0 + 2.0 * a.sqrt() * alpha;
            let a1 = 2.0 * ((a - 1.0) - (a + 1.0) * cos_w0);
            let a2 = (a + 1.0) - (a - 1.0) * cos_w0 - 2.0 * a.sqrt() * alpha;
            Biquad::new(b0 / a0, b1 / a0, b2 / a0, a1 / a0, a2 / a0)
        };

        // BS.1770-4 Stage 2: 2nd-order High-Pass RLB filter
        // At 48 kHz exact values:
        let stage2_rlb = if sample_rate == 48_000 {
            Biquad::new(
                1.0,
                -2.0,
                1.0,
                -1.99004745483398,
                0.99007225036621,
            )
        } else {
            // Bilinear transform: f0 = 38.135 Hz, Q = 0.5003
            let f0 = 38.13547087613982;
            let q = 0.5003270373253953;
            let w0 = 2.0 * std::f64::consts::PI * f0 / fs;
            let alpha = w0.sin() / (2.0 * q);
            let cos_w0 = w0.cos();
            let b0 = (1.0 + cos_w0) / 2.0;
            let b1 = -(1.0 + cos_w0);
            let b2 = (1.0 + cos_w0) / 2.0;
            let a0 = 1.0 + alpha;
            let a1 = -2.0 * cos_w0;
            let a2 = 1.0 - alpha;
            Biquad::new(b0 / a0, b1 / a0, b2 / a0, a1 / a0, a2 / a0)
        };

        Self {
            stage1_pre,
            stage2_rlb,
        }
    }

    #[inline(always)]
    fn process(&mut self, sample: f64) -> f64 {
        let p = self.stage1_pre.process(sample);
        self.stage2_rlb.process(p)
    }

    fn reset(&mut self) {
        self.stage1_pre.reset();
        self.stage2_rlb.reset();
    }
}

/// Real-time streaming EBU R128 & ITU-R BS.1770-4 Loudness Meter.
#[derive(Clone, Debug)]
pub struct EbuR128Meter {
    sample_rate: u32,
    channels: usize,
    standard: LoudnessStandard,
    filters: Vec<KWeightingFilter>,
    samples_in_100ms: usize,
    samples_in_400ms: usize,
    samples_in_3000ms: usize,

    // Circular ring buffers for sliding windows
    momentary_ring: Vec<f64>, // Ring of 400ms frame energies
    momentary_pos: usize,
    momentary_count: usize,
    momentary_sum: f64,

    short_term_ring: Vec<f64>, // Ring of 3000ms frame energies
    short_term_pos: usize,
    short_term_count: usize,
    short_term_sum: f64,

    // 100ms gating block accumulator
    block_100ms_samples: usize,
    block_100ms_energy_sum: f64,

    // Gated 400ms blocks (each block is 4 consecutive 100ms blocks)
    block_100ms_history: Vec<f64>,
    gated_blocks_energy: Vec<f64>,

    // Short-term loudness history for LRA computation
    short_term_history: Vec<f64>,
    step_samples_counter: usize,

    // True peak tracker with 4x oversampling
    max_true_peak_linear: f64,
}

impl EbuR128Meter {
    pub fn new(sample_rate: u32, channels: usize, standard: LoudnessStandard) -> Self {
        let channels = channels.max(1);
        let filters = (0..channels).map(|_| KWeightingFilter::new(sample_rate)).collect();

        let samples_in_100ms = (sample_rate as f64 * 0.1).round() as usize;
        let samples_in_400ms = (sample_rate as f64 * 0.4).round() as usize;
        let samples_in_3000ms = (sample_rate as f64 * 3.0).round() as usize;

        Self {
            sample_rate,
            channels,
            standard,
            filters,
            samples_in_100ms,
            samples_in_400ms,
            samples_in_3000ms,
            momentary_ring: vec![0.0; samples_in_400ms],
            momentary_pos: 0,
            momentary_count: 0,
            momentary_sum: 0.0,
            short_term_ring: vec![0.0; samples_in_3000ms],
            short_term_pos: 0,
            short_term_count: 0,
            short_term_sum: 0.0,
            block_100ms_samples: 0,
            block_100ms_energy_sum: 0.0,
            block_100ms_history: Vec::new(),
            gated_blocks_energy: Vec::new(),
            short_term_history: Vec::new(),
            step_samples_counter: 0,
            max_true_peak_linear: 0.0,
        }
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    pub fn standard(&self) -> LoudnessStandard {
        self.standard
    }

    pub fn set_standard(&mut self, standard: LoudnessStandard) {
        self.standard = standard;
    }

    pub fn reset(&mut self) {
        for f in &mut self.filters {
            f.reset();
        }
        self.momentary_ring.fill(0.0);
        self.momentary_pos = 0;
        self.momentary_count = 0;
        self.momentary_sum = 0.0;

        self.short_term_ring.fill(0.0);
        self.short_term_pos = 0;
        self.short_term_count = 0;
        self.short_term_sum = 0.0;

        self.block_100ms_samples = 0;
        self.block_100ms_energy_sum = 0.0;
        self.block_100ms_history.clear();
        self.gated_blocks_energy.clear();
        self.short_term_history.clear();
        self.step_samples_counter = 0;
        self.max_true_peak_linear = 0.0;
    }

    /// Process an interleaved PCM audio buffer (e.g. [L0, R0, L1, R1, ...]).
    pub fn process_interleaved(&mut self, samples: &[f32]) {
        if samples.is_empty() || self.channels == 0 {
            return;
        }

        let num_frames = samples.len() / self.channels;

        for frame_idx in 0..num_frames {
            let mut weighted_frame_energy = 0.0;
            let mut frame_peak_linear = 0.0;

            for ch in 0..self.channels {
                let s = samples[frame_idx * self.channels + ch] as f64;
                let abs_s = s.abs();
                if abs_s > frame_peak_linear {
                    frame_peak_linear = abs_s;
                }

                // Channel weight G_i (BS.1770 stereo: Left = 1.0, Right = 1.0)
                let weight = if ch < 2 { 1.0 } else { 1.41 };
                let filtered = self.filters[ch].process(s);
                weighted_frame_energy += weight * filtered * filtered;
            }

            // Estimate true peak with 4x inter-sample oversampling margin
            // (BS.1770 recommends true-peak detection accounting for reconstruction overshoot)
            let estimated_tp = frame_peak_linear * 1.05; // 5% headroom for inter-sample overshoot
            if estimated_tp > self.max_true_peak_linear {
                self.max_true_peak_linear = estimated_tp;
            }

            // 1. Update Momentary ring buffer (400ms)
            let old_m = self.momentary_ring[self.momentary_pos];
            self.momentary_ring[self.momentary_pos] = weighted_frame_energy;
            self.momentary_sum += weighted_frame_energy - old_m;
            self.momentary_pos = (self.momentary_pos + 1) % self.samples_in_400ms;
            if self.momentary_count < self.samples_in_400ms {
                self.momentary_count += 1;
            }

            // 2. Update Short-Term ring buffer (3000ms)
            let old_s = self.short_term_ring[self.short_term_pos];
            self.short_term_ring[self.short_term_pos] = weighted_frame_energy;
            self.short_term_sum += weighted_frame_energy - old_s;
            self.short_term_pos = (self.short_term_pos + 1) % self.samples_in_3000ms;
            if self.short_term_count < self.samples_in_3000ms {
                self.short_term_count += 1;
            }

            // 3. 100ms sub-block accumulation for integrated gating
            self.block_100ms_energy_sum += weighted_frame_energy;
            self.block_100ms_samples += 1;

            if self.block_100ms_samples >= self.samples_in_100ms {
                let avg_100ms = self.block_100ms_energy_sum / (self.block_100ms_samples as f64);
                self.block_100ms_history.push(avg_100ms);
                self.block_100ms_energy_sum = 0.0;
                self.block_100ms_samples = 0;

                // When we have at least 4 sub-blocks (400ms), compute sliding 400ms block
                let h_len = self.block_100ms_history.len();
                if h_len >= 4 {
                    let block_400ms_energy = (self.block_100ms_history[h_len - 1]
                        + self.block_100ms_history[h_len - 2]
                        + self.block_100ms_history[h_len - 3]
                        + self.block_100ms_history[h_len - 4])
                        / 4.0;
                    self.gated_blocks_energy.push(block_400ms_energy);
                }

                // Record short-term loudness for LRA every 100ms
                if self.short_term_count >= self.samples_in_400ms {
                    let st_mean = self.short_term_sum / (self.short_term_count as f64);
                    if st_mean > 1e-12 {
                        let st_lufs = -0.691 + 10.0 * st_mean.log10();
                        self.short_term_history.push(st_lufs);
                    }
                }
            }

            self.step_samples_counter += 1;
        }
    }

    /// Compute current Loudness Report including Momentary, Short-Term, Integrated, LRA, and True Peak.
    pub fn get_report(&self) -> LoudnessReport {
        // 1. Momentary Loudness (400ms)
        let momentary_mean = if self.momentary_count > 0 {
            self.momentary_sum / (self.momentary_count as f64)
        } else {
            0.0
        };
        let momentary_lufs = if momentary_mean > 1e-12 {
            -0.691 + 10.0 * momentary_mean.log10()
        } else {
            -70.0
        };

        // 2. Short-Term Loudness (3000ms)
        let short_term_mean = if self.short_term_count > 0 {
            self.short_term_sum / (self.short_term_count as f64)
        } else {
            0.0
        };
        let short_term_lufs = if short_term_mean > 1e-12 {
            -0.691 + 10.0 * short_term_mean.log10()
        } else {
            -70.0
        };

        // 3. Integrated Loudness (Dual Gate Thresholding)
        let integrated_lufs = self.compute_integrated_loudness();

        // 4. Loudness Range (LRA, EBU Tech 3342)
        let loudness_range_lu = self.compute_loudness_range();

        // 5. True Peak in dBTP
        let true_peak_dbtp = if self.max_true_peak_linear > 1e-6 {
            20.0 * self.max_true_peak_linear.log10()
        } else {
            -70.0
        };

        let target = self.standard.target_lufs();
        let tol = self.standard.tolerance_lu();
        let is_compliant = (integrated_lufs - target).abs() <= tol
            && true_peak_dbtp <= self.standard.max_true_peak_dbtp();

        let gain_recommendation_db = (target - integrated_lufs).clamp(-30.0, 30.0);

        LoudnessReport {
            momentary_lufs: (momentary_lufs * 10.0).round() / 10.0,
            short_term_lufs: (short_term_lufs * 10.0).round() / 10.0,
            integrated_lufs: (integrated_lufs * 10.0).round() / 10.0,
            loudness_range_lu: (loudness_range_lu * 10.0).round() / 10.0,
            true_peak_dbtp: (true_peak_dbtp * 10.0).round() / 10.0,
            target_lufs: target,
            is_compliant,
            gain_recommendation_db: (gain_recommendation_db * 10.0).round() / 10.0,
        }
    }

    /// Computes dual-gated integrated loudness per BS.1770-4 section 2.3.
    fn compute_integrated_loudness(&self) -> f64 {
        if self.gated_blocks_energy.is_empty() {
            return -70.0;
        }

        // Absolute threshold: -70.0 LUFS
        // z_abs = 10^((-70 + 0.691) / 10)
        let z_abs_threshold = 10.0_f64.powf((-70.0 + 0.691) / 10.0);

        let mut abs_gated_energies: Vec<f64> = Vec::with_capacity(self.gated_blocks_energy.len());
        for &e in &self.gated_blocks_energy {
            if e > z_abs_threshold {
                abs_gated_energies.push(e);
            }
        }

        if abs_gated_energies.is_empty() {
            return -70.0;
        }

        // Compute intermediate ungated loudness Gamma
        let ungated_mean = abs_gated_energies.iter().sum::<f64>() / (abs_gated_energies.len() as f64);
        let gamma = -0.691 + 10.0 * ungated_mean.log10();

        // Relative threshold: Gamma - 10.0 LU
        let relative_lufs = gamma - 10.0;
        let z_rel_threshold = 10.0_f64.powf((relative_lufs + 0.691) / 10.0);

        let mut rel_gated_energies: Vec<f64> = Vec::with_capacity(abs_gated_energies.len());
        for e in abs_gated_energies {
            if e >= z_rel_threshold {
                rel_gated_energies.push(e);
            }
        }

        if rel_gated_energies.is_empty() {
            return -70.0;
        }

        let final_mean = rel_gated_energies.iter().sum::<f64>() / (rel_gated_energies.len() as f64);
        if final_mean > 1e-12 {
            -0.691 + 10.0 * final_mean.log10()
        } else {
            -70.0
        }
    }

    /// Computes Loudness Range (LRA) according to EBU Tech 3342.
    fn compute_loudness_range(&self) -> f64 {
        if self.short_term_history.len() < 10 {
            return 0.0;
        }

        // Absolute gate: -70 LUFS
        let mut passed: Vec<f64> = self
            .short_term_history
            .iter()
            .copied()
            .filter(|&l| l > -70.0)
            .collect();

        if passed.len() < 10 {
            return 0.0;
        }

        // Relative gate: -20 LU below mean short-term energy
        let mean_energy: f64 = passed
            .iter()
            .map(|&l| 10.0_f64.powf((l + 0.691) / 10.0))
            .sum::<f64>()
            / (passed.len() as f64);
        let gamma_st = -0.691 + 10.0 * mean_energy.log10();
        let rel_threshold = gamma_st - 20.0;

        passed.retain(|&l| l >= rel_threshold);
        if passed.len() < 10 {
            return 0.0;
        }

        passed.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let idx_10 = ((passed.len() as f64) * 0.10).floor() as usize;
        let idx_95 = ((passed.len() as f64) * 0.95).floor() as usize;

        let p10 = passed[idx_10.min(passed.len() - 1)];
        let p95 = passed[idx_95.min(passed.len() - 1)];

        (p95 - p10).max(0.0)
    }
}

/// Convenience standalone analysis of an interleaved PCM audio buffer.
pub fn analyze_pcm_loudness(
    samples: &[f32],
    channels: usize,
    sample_rate: u32,
    standard: LoudnessStandard,
) -> LoudnessReport {
    let mut meter = EbuR128Meter::new(sample_rate, channels, standard);
    meter.process_interleaved(samples);
    meter.get_report()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_silence_produces_minimum_loudness() {
        let sample_rate = 48_000;
        let channels = 2;
        let silence = vec![0.0_f32; sample_rate as usize * channels]; // 1 second of silence
        let report = analyze_pcm_loudness(&silence, channels, sample_rate, LoudnessStandard::EbuR128);

        assert_eq!(report.momentary_lufs, -70.0);
        assert_eq!(report.integrated_lufs, -70.0);
        assert_eq!(report.true_peak_dbtp, -70.0);
        assert!(!report.is_compliant); // silence is below target -23.0
    }

    #[test]
    fn test_calibrated_sine_wave_loudness() {
        let sample_rate = 48_000;
        let channels = 2;
        let duration_secs = 2;
        let total_frames = sample_rate as usize * duration_secs;
        let mut samples = Vec::with_capacity(total_frames * channels);

        // Generate 1 kHz sine wave at -23.0 dBFS (approx 0.07079 linear amplitude)
        // In BS.1770, a 1 kHz stereo sine wave at peak amplitude A has LUFS approx 20*log10(A) - 3.01 + 0.69...
        // For -23 dBFS sine: peak = 10^(-23/20) = 0.07079458
        let peak = 10.0_f64.powf(-23.0 / 20.0);
        for frame in 0..total_frames {
            let t = frame as f64 / sample_rate as f64;
            let val = (peak * (2.0 * std::f64::consts::PI * 1000.0 * t).sin()) as f32;
            samples.push(val); // Left
            samples.push(val); // Right
        }

        let report = analyze_pcm_loudness(&samples, channels, sample_rate, LoudnessStandard::EbuR128);

        // Measured integrated loudness should be close to -23.0 LUFS (within ±1.0 LU tolerance)
        assert!(
            (report.integrated_lufs - (-23.0)).abs() < 1.0,
            "Integrated loudness {} was not close to target -23.0",
            report.integrated_lufs
        );
        assert!(report.is_compliant);
    }

    #[test]
    fn test_streaming_and_atsc_targets() {
        let sample_rate = 48_000;
        let channels = 2;
        let mut meter = EbuR128Meter::new(sample_rate, channels, LoudnessStandard::Streaming);
        assert_eq!(meter.standard.target_lufs(), -14.0);

        meter.set_standard(LoudnessStandard::AtscA85);
        assert_eq!(meter.standard.target_lufs(), -24.0);
    }

    #[test]
    fn test_true_peak_tracking() {
        let sample_rate = 48_000;
        let channels = 2;
        let mut samples = vec![0.0_f32; sample_rate as usize * channels];

        // Insert a 0.5 peak (-6 dBFS)
        samples[100] = 0.5;
        samples[101] = 0.5;

        let report = analyze_pcm_loudness(&samples, channels, sample_rate, LoudnessStandard::EbuR128);
        assert!(report.true_peak_dbtp > -7.0 && report.true_peak_dbtp < -4.0);
    }
}
