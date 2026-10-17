use saturn_core::{TICKS_PER_SECOND, Tick};

/// Cross-correlates two audio waveform peak sequences to find the optimal sync offset.
/// Returns `(best_lag_ticks, confidence)`.
pub fn sync_waveforms_by_correlation(
    wf_reference: &[u8],
    wf_target: &[u8],
    peaks_per_second: usize,
) -> (Tick, f64) {
    if wf_reference.is_empty() || wf_target.is_empty() {
        return (Tick(0), 0.0);
    }

    let n_ref = wf_reference.len();
    let n_tgt = wf_target.len();

    // Center the signals (remove DC bias)
    let mean_ref = wf_reference.iter().map(|&x| x as f64).sum::<f64>() / n_ref as f64;
    let mean_tgt = wf_target.iter().map(|&x| x as f64).sum::<f64>() / n_tgt as f64;

    let centered_ref: Vec<f64> = wf_reference.iter().map(|&x| x as f64 - mean_ref).collect();
    let centered_tgt: Vec<f64> = wf_target.iter().map(|&x| x as f64 - mean_tgt).collect();

    // Max search window
    let max_lag = (n_ref.min(n_tgt) / 2).max(1);
    let mut best_lag = 0isize;
    let mut max_corr = f64::NEG_INFINITY;

    for lag in -(max_lag as isize)..=(max_lag as isize) {
        let mut dot = 0.0;
        let mut norm_ref = 0.0;
        let mut norm_tgt = 0.0;

        for i in 0..n_ref {
            let j = i as isize + lag;
            if j >= 0 && (j as usize) < n_tgt {
                let vr = centered_ref[i];
                let vt = centered_tgt[j as usize];
                dot += vr * vt;
                norm_ref += vr * vr;
                norm_tgt += vt * vt;
            }
        }

        let denom = (norm_ref * norm_tgt).sqrt();
        if denom > 1e-6 {
            let corr = dot / denom;
            if corr > max_corr {
                max_corr = corr;
                best_lag = lag;
            }
        }
    }

    let pps = peaks_per_second.max(1) as f64;
    let seconds_offset = (best_lag as f64) / pps;
    let ticks_offset = (seconds_offset.abs() * TICKS_PER_SECOND as f64) as i64;
    let confidence = max_corr.clamp(0.0, 1.0);

    (Tick(ticks_offset), confidence)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identical_waveforms_sync_at_zero() {
        let mut wf = Vec::new();
        for i in 0..100 {
            let val = ((i as f64 * 0.2).sin() * 50.0 + 128.0) as u8;
            wf.push(val);
        }

        let (offset, confidence) = sync_waveforms_by_correlation(&wf, &wf, 10);
        assert_eq!(offset, Tick(0));
        assert!(confidence > 0.95);
    }

    #[test]
    fn test_shifted_waveform_detects_lag() {
        let mut wf1 = Vec::new();
        let mut wf2 = vec![128u8; 10]; // 10 peak delay (1 second at 10 pps)

        for i in 0..80 {
            let val = ((i as f64 * 0.3).sin() * 60.0 + 128.0) as u8;
            wf1.push(val);
            wf2.push(val);
        }

        let (offset, confidence) = sync_waveforms_by_correlation(&wf1, &wf2, 10);
        assert!(confidence > 0.8);
        // ~1 second in ticks is ~254_016_000_000
        assert!((offset.0 - TICKS_PER_SECOND as i64).abs() < (TICKS_PER_SECOND as i64 / 2));
    }
}
