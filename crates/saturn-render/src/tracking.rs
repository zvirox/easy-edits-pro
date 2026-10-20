use saturn_core::Tick;
use saturn_core::project::TrackPoint;

/// Smooths a sequence of trajectory points using a moving average window.
/// `smoothness` ranges from 0.0 (no smoothing) to 1.0 (heavy smoothing).
pub fn smooth_trajectory(points: &[TrackPoint], smoothness: f64) -> Vec<TrackPoint> {
    if points.is_empty() {
        return Vec::new();
    }
    if points.len() == 1 || smoothness <= 0.0 {
        return points.to_vec();
    }

    let window_radius = ((smoothness.clamp(0.0, 1.0) * 10.0).round() as usize).max(1);
    let mut smoothed = Vec::with_capacity(points.len());

    for i in 0..points.len() {
        let start = i.saturating_sub(window_radius);
        let end = (i + window_radius + 1).min(points.len());
        let slice = &points[start..end];

        let mut sum_x = 0.0;
        let mut sum_y = 0.0;
        let mut sum_w = 0.0;

        for (idx, pt) in slice.iter().enumerate() {
            let dist = (idx as isize - (i - start) as isize).abs() as f64;
            let weight = (-dist * dist / (2.0 * (window_radius as f64).powi(2) + 0.1)).exp();
            let conf_weight = weight * pt.confidence.clamp(0.1, 1.0);
            sum_x += pt.x * conf_weight;
            sum_y += pt.y * conf_weight;
            sum_w += conf_weight;
        }

        let avg_x = if sum_w > 0.0 {
            sum_x / sum_w
        } else {
            points[i].x
        };
        let avg_y = if sum_w > 0.0 {
            sum_y / sum_w
        } else {
            points[i].y
        };

        smoothed.push(TrackPoint {
            time: points[i].time,
            x: avg_x,
            y: avg_y,
            confidence: points[i].confidence,
        });
    }

    smoothed
}

/// Calculates camera stabilization offset (delta_x, delta_y) that cancels high-frequency jitter.
/// When added to clip position, high-frequency camera shake is neutralized.
pub fn compute_stabilization_offset(point: &TrackPoint, smoothed: &TrackPoint) -> (f64, f64) {
    let delta_x = smoothed.x - point.x;
    let delta_y = smoothed.y - point.y;
    (delta_x, delta_y)
}

/// Interpolates position (x, y) along a tracking trajectory at a given timeline `tick`.
pub fn evaluate_tracking_at(points: &[TrackPoint], tick: Tick) -> Option<(f64, f64)> {
    if points.is_empty() {
        return None;
    }
    if points.len() == 1 || tick.0 <= points[0].time.0 {
        return Some((points[0].x, points[0].y));
    }
    if tick.0 >= points.last()?.time.0 {
        let last = points.last()?;
        return Some((last.x, last.y));
    }

    let next_idx = points.iter().position(|p| p.time.0 >= tick.0)?;
    let prev = &points[next_idx - 1];
    let next = &points[next_idx];

    let dt = (next.time.0 - prev.time.0) as f64;
    let t = if dt > 0.0 {
        (tick.0 - prev.time.0) as f64 / dt
    } else {
        0.0
    };

    let x = prev.x + (next.x - prev.x) * t;
    let y = prev.y + (next.y - prev.y) * t;
    Some((x, y))
}

/// Estimates optical displacement between two consecutive grayscale patches using
/// Lucas-Kanade block matching / sum of squared differences (SSD).
pub fn track_patch_displacement(
    prev_patch: &[u8],
    curr_patch: &[u8],
    patch_size: usize,
    max_search_radius: usize,
) -> (f64, f64, f64) {
    if prev_patch.len() < patch_size * patch_size || curr_patch.len() < patch_size * patch_size {
        return (0.0, 0.0, 0.0);
    }

    let mut best_dx = 0isize;
    let mut best_dy = 0isize;
    let mut min_ssd = u64::MAX;

    let r = max_search_radius as isize;
    for dy in -r..=r {
        for dx in -r..=r {
            let mut ssd = 0u64;
            let mut count = 0u64;

            for py in 0..patch_size {
                let cy = py as isize + dy;
                if cy < 0 || cy >= patch_size as isize {
                    continue;
                }
                for px in 0..patch_size {
                    let cx = px as isize + dx;
                    if cx < 0 || cx >= patch_size as isize {
                        continue;
                    }

                    let p_val = prev_patch[py * patch_size + px] as i64;
                    let c_val = curr_patch[(cy as usize) * patch_size + (cx as usize)] as i64;
                    let diff = p_val - c_val;
                    ssd += (diff * diff) as u64;
                    count += 1;
                }
            }

            if count > 0 && ssd < min_ssd {
                min_ssd = ssd;
                best_dx = dx;
                best_dy = dy;
            }
        }
    }

    let confidence = if min_ssd < 1000 {
        0.98
    } else if min_ssd < 10000 {
        0.85
    } else {
        0.5
    };

    (best_dx as f64, best_dy as f64, confidence)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_smooth_trajectory_reduces_variance() {
        let raw = vec![
            TrackPoint {
                time: Tick(0),
                x: 0.50,
                y: 0.50,
                confidence: 1.0,
            },
            TrackPoint {
                time: Tick(10),
                x: 0.55,
                y: 0.45,
                confidence: 1.0,
            }, // jitter
            TrackPoint {
                time: Tick(20),
                x: 0.51,
                y: 0.51,
                confidence: 1.0,
            },
            TrackPoint {
                time: Tick(30),
                x: 0.56,
                y: 0.44,
                confidence: 1.0,
            }, // jitter
            TrackPoint {
                time: Tick(40),
                x: 0.52,
                y: 0.52,
                confidence: 1.0,
            },
        ];

        let smoothed = smooth_trajectory(&raw, 0.8);
        assert_eq!(smoothed.len(), raw.len());
        // Midpoint should be closer to true average (~0.52) than jitter (~0.56)
        assert!((smoothed[3].x - 0.52).abs() < (raw[3].x - 0.52).abs());
    }

    #[test]
    fn test_evaluate_tracking_interpolation() {
        let pts = vec![
            TrackPoint {
                time: Tick(0),
                x: 100.0,
                y: 200.0,
                confidence: 1.0,
            },
            TrackPoint {
                time: Tick(100),
                x: 200.0,
                y: 300.0,
                confidence: 1.0,
            },
        ];

        let mid = evaluate_tracking_at(&pts, Tick(50)).unwrap();
        assert_eq!(mid, (150.0, 250.0));
    }

    #[test]
    fn test_stabilization_offset_neutralizes_shake() {
        let jitter_pt = TrackPoint {
            time: Tick(10),
            x: 0.55,
            y: 0.48,
            confidence: 1.0,
        };
        let smoothed_pt = TrackPoint {
            time: Tick(10),
            x: 0.50,
            y: 0.50,
            confidence: 1.0,
        };

        let (dx, dy) = compute_stabilization_offset(&jitter_pt, &smoothed_pt);
        // Corrected point: jitter + offset == smoothed
        assert!((jitter_pt.x + dx - smoothed_pt.x).abs() < 1e-6);
        assert!((jitter_pt.y + dy - smoothed_pt.y).abs() < 1e-6);
    }
}
