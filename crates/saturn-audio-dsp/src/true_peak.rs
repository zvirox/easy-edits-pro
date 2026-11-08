// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Four-times windowed-sinc reconstruction for inter-sample peak measurement.
//! A 65-tap Blackman FIR per fractional phase; includes original sample peaks.
use std::f64::consts::PI;

const TAPS: usize = 65;
const CENTER: usize = 32;

#[derive(Clone, Debug)]
pub struct TruePeakMeter {
    // Duplicate the ring to make the convolution contiguous (no modulo in
    // its inner loop; material on the 120-second offline processing path).
    history: Vec<[f64; TAPS * 2]>,
    kernels: [[f64; TAPS]; 3],
    position: usize,
    peak: f64,
}

impl TruePeakMeter {
    pub fn new(channels: usize) -> Self {
        let mut kernels = [[0.0; TAPS]; 3];
        for (p, kernel) in kernels.iter_mut().enumerate() {
            for (i, tap) in kernel.iter_mut().enumerate() {
                let x = i as f64 - CENTER as f64 - (p + 1) as f64 / 4.0;
                let window =
                    0.42 + 0.5 * (PI * x / 33.0).cos() + 0.08 * (2.0 * PI * x / 33.0).cos();
                *tap = (PI * x).sin() / (PI * x) * window;
            }
            let sum: f64 = kernel.iter().sum();
            for tap in kernel {
                *tap /= sum;
            }
        }
        Self {
            history: vec![[0.0; TAPS * 2]; channels],
            kernels,
            position: 0,
            peak: 0.0,
        }
    }

    pub fn process_frame(&mut self, frame: &[f32]) {
        assert_eq!(frame.len(), self.history.len());
        for (history, &sample) in self.history.iter_mut().zip(frame) {
            history[self.position] = sample as f64;
            history[self.position + TAPS] = sample as f64;
            self.peak = self.peak.max((sample as f64).abs());
            for kernel in &self.kernels {
                let value: f64 = kernel
                    .iter()
                    .zip(&history[self.position + 1..self.position + 1 + TAPS])
                    .map(|(tap, value)| tap * value)
                    .sum();
                self.peak = self.peak.max(value.abs());
            }
        }
        self.position = (self.position + 1) % TAPS;
    }

    /// Finish the FIR tail on a clone so reading a meter never changes state.
    pub fn peak_linear(&self) -> f64 {
        let mut finished = self.clone();
        let silence = vec![0.0; self.history.len()];
        for _ in 0..TAPS {
            finished.process_frame(&silence);
        }
        finished.peak
    }
}

pub fn measure_peak(samples: &[f32], channels: usize) -> Result<f64, String> {
    if channels == 0
        || channels > 8
        || samples.len() % channels != 0
        || samples.iter().any(|x| !x.is_finite())
    {
        return Err("True-peak measurement requires finite interleaved PCM (1..8 channels)".into());
    }
    let mut meter = TruePeakMeter::new(channels);
    for frame in samples.chunks_exact(channels) {
        meter.process_frame(frame);
    }
    Ok(meter.peak_linear())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reconstructs_peak_between_samples() {
        // fs/4 with pi/4 phase has sample peak 0.7071 and reconstructed peak 1.
        let samples: Vec<_> = (0..4800)
            .map(|i| (PI * i as f64 / 2.0 + PI / 4.0).sin() as f32)
            .collect();
        let peak = measure_peak(&samples, 1).unwrap();
        assert!(peak > 0.99 && peak < 1.08, "{peak}");
        assert!(peak > samples.iter().copied().map(f32::abs).fold(0.0, f32::max) as f64 * 1.3);
    }
    #[test]
    fn chunking_and_reads_do_not_change_measurement() {
        let mut meter = TruePeakMeter::new(1);
        for &v in &[0.0, 1.0, -1.0, 0.0] {
            meter.process_frame(&[v]);
            let _ = meter.peak_linear();
        }
        assert_eq!(
            meter.peak_linear(),
            measure_peak(&[0.0, 1.0, -1.0, 0.0], 1).unwrap()
        );
    }
}
