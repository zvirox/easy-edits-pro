use saturn_core::Tick;
use saturn_core::project::SpeedRamp;

/// Evaluates instantaneous speed factor of a SpeedRamp curve at `local_tick`.
pub fn interpolate_speed_ramp(ramp: &SpeedRamp, local_tick: Tick) -> f64 {
    if ramp.points.is_empty() {
        return 1.0;
    }
    if ramp.points.len() == 1 {
        return ramp.points[0].speed_factor.max(0.01);
    }

    if local_tick.0 <= ramp.points[0].time.0 {
        return ramp.points[0].speed_factor.max(0.01);
    }
    let last = ramp.points.last().unwrap();
    if local_tick.0 >= last.time.0 {
        return last.speed_factor.max(0.01);
    }

    // Find segment between point i and i+1
    for i in 0..ramp.points.len() - 1 {
        let p0 = &ramp.points[i];
        let p1 = &ramp.points[i + 1];
        if local_tick.0 >= p0.time.0 && local_tick.0 <= p1.time.0 {
            let seg_dur = (p1.time.0 - p0.time.0).max(1) as f64;
            let t = ((local_tick.0 - p0.time.0) as f64 / seg_dur).clamp(0.0, 1.0);

            // Cubic Hermite / Bezier tangent interpolation
            let h00 = 2.0 * t.powi(3) - 3.0 * t.powi(2) + 1.0;
            let h10 = t.powi(3) - 2.0 * t.powi(2) + t;
            let h01 = -2.0 * t.powi(3) + 3.0 * t.powi(2);
            let h11 = t.powi(3) - t.powi(2);

            let m0 = p0.bezier_tangent.1 * (p1.speed_factor - p0.speed_factor);
            let m1 = p1.bezier_tangent.0 * (p1.speed_factor - p0.speed_factor);

            let val = h00 * p0.speed_factor + h10 * m0 + h01 * p1.speed_factor + h11 * m1;
            return val.max(0.01);
        }
    }

    1.0
}

/// Bidirectional optical flow frame synthesis.
/// Warps `prev_frame` forward and `next_frame` backward using motion vectors,
/// blending them with weight `alpha` to produce smooth, non-stuttering slow motion frames.
pub fn blend_bidirectional_optical_flow(
    prev_frame: &[u8],
    next_frame: &[u8],
    forward_flow: Option<&[(f32, f32)]>,
    backward_flow: Option<&[(f32, f32)]>,
    alpha: f32,
    width: usize,
    height: usize,
) -> Vec<u8> {
    let alpha = alpha.clamp(0.0, 1.0);
    let pixel_count = width * height;
    let mut out = vec![0u8; pixel_count * 4];

    if alpha <= 0.001 {
        out.copy_from_slice(prev_frame);
        return out;
    }
    if alpha >= 0.999 {
        out.copy_from_slice(next_frame);
        return out;
    }

    // If explicit flows are not supplied, compute fast block-based optical flow vectors
    let computed_fwd;
    let computed_bwd;
    let (fwd, bwd) = match (forward_flow, backward_flow) {
        (Some(f), Some(b)) => (f, b),
        _ => {
            computed_fwd = estimate_sparse_flow(prev_frame, next_frame, width, height);
            computed_bwd = estimate_sparse_flow(next_frame, prev_frame, width, height);
            (computed_fwd.as_slice(), computed_bwd.as_slice())
        }
    };

    // Synthesize intermediate frame
    for y in 0..height {
        for x in 0..width {
            let idx = y * width + x;
            let (u_fwd, v_fwd) = fwd.get(idx).copied().unwrap_or((0.0, 0.0));
            let (u_bwd, v_bwd) = bwd.get(idx).copied().unwrap_or((0.0, 0.0));

            // Forward warp: where did pixel (x, y) come from in prev_frame?
            let src_fwd_x = (x as f32 - alpha * u_fwd).clamp(0.0, (width - 1) as f32);
            let src_fwd_y = (y as f32 - alpha * v_fwd).clamp(0.0, (height - 1) as f32);
            let sample_fwd = sample_bilinear_rgba(prev_frame, src_fwd_x, src_fwd_y, width, height);

            // Backward warp: where does pixel (x, y) project from in next_frame?
            let src_bwd_x = (x as f32 + (1.0 - alpha) * u_bwd).clamp(0.0, (width - 1) as f32);
            let src_bwd_y = (y as f32 + (1.0 - alpha) * v_bwd).clamp(0.0, (height - 1) as f32);
            let sample_bwd = sample_bilinear_rgba(next_frame, src_bwd_x, src_bwd_y, width, height);

            let out_idx = idx * 4;
            let w_fwd = 1.0 - alpha;
            let w_bwd = alpha;

            out[out_idx] = ((sample_fwd[0] as f32 * w_fwd + sample_bwd[0] as f32 * w_bwd).round()
                as u8)
                .min(255);
            out[out_idx + 1] = ((sample_fwd[1] as f32 * w_fwd + sample_bwd[1] as f32 * w_bwd)
                .round() as u8)
                .min(255);
            out[out_idx + 2] = ((sample_fwd[2] as f32 * w_fwd + sample_bwd[2] as f32 * w_bwd)
                .round() as u8)
                .min(255);
            out[out_idx + 3] = ((sample_fwd[3] as f32 * w_fwd + sample_bwd[3] as f32 * w_bwd)
                .round() as u8)
                .min(255);
        }
    }

    out
}

fn sample_bilinear_rgba(frame: &[u8], x: f32, y: f32, width: usize, height: usize) -> [u8; 4] {
    let x0 = (x.floor() as usize).min(width - 1);
    let y0 = (y.floor() as usize).min(height - 1);
    let x1 = (x0 + 1).min(width - 1);
    let y1 = (y0 + 1).min(height - 1);

    let fx = x - x.floor();
    let fy = y - y.floor();

    let i00 = (y0 * width + x0) * 4;
    let i10 = (y0 * width + x1) * 4;
    let i01 = (y1 * width + x0) * 4;
    let i11 = (y1 * width + x1) * 4;

    let mut result = [0u8; 4];
    for c in 0..4 {
        let v00 = frame[i00 + c] as f32;
        let v10 = frame[i10 + c] as f32;
        let v01 = frame[i01 + c] as f32;
        let v11 = frame[i11 + c] as f32;

        let top = v00 * (1.0 - fx) + v10 * fx;
        let btm = v01 * (1.0 - fx) + v11 * fx;
        let val = top * (1.0 - fy) + btm * fy;
        result[c] = val.round().clamp(0.0, 255.0) as u8;
    }
    result
}

/// Fast optical flow estimation based on local luminance patch matching.
fn estimate_sparse_flow(src: &[u8], dst: &[u8], width: usize, height: usize) -> Vec<(f32, f32)> {
    let pixel_count = width * height;
    let mut flow = vec![(0.0f32, 0.0f32); pixel_count];
    let block_size = 8;
    let search_radius = 4;

    for by in (0..height).step_by(block_size) {
        for bx in (0..width).step_by(block_size) {
            let mut best_dx = 0;
            let mut best_dy = 0;
            let mut min_sad = u64::MAX;

            for dy in -search_radius..=search_radius {
                for dx in -search_radius..=search_radius {
                    let mut sad: u64 = 0;
                    for iy in 0..block_size {
                        let sy = by + iy;
                        let ty = (sy as isize + dy).clamp(0, (height - 1) as isize) as usize;
                        if sy >= height {
                            break;
                        }

                        for ix in 0..block_size {
                            let sx = bx + ix;
                            let tx = (sx as isize + dx).clamp(0, (width - 1) as isize) as usize;
                            if sx >= width {
                                break;
                            }

                            let s_idx = (sy * width + sx) * 4;
                            let t_idx = (ty * width + tx) * 4;
                            // Luminance difference
                            let s_luma = (src[s_idx] as u32 * 54
                                + src[s_idx + 1] as u32 * 183
                                + src[s_idx + 2] as u32 * 19)
                                >> 8;
                            let t_luma = (dst[t_idx] as u32 * 54
                                + dst[t_idx + 1] as u32 * 183
                                + dst[t_idx + 2] as u32 * 19)
                                >> 8;
                            sad += s_luma.abs_diff(t_luma) as u64;
                        }
                    }

                    if sad < min_sad {
                        min_sad = sad;
                        best_dx = dx;
                        best_dy = dy;
                    }
                }
            }

            // Assign flow to all pixels in block
            for iy in 0..block_size {
                let y = by + iy;
                if y >= height {
                    break;
                }
                for ix in 0..block_size {
                    let x = bx + ix;
                    if x >= width {
                        break;
                    }
                    flow[y * width + x] = (best_dx as f32, best_dy as f32);
                }
            }
        }
    }

    flow
}

/// Dynamic shutter angle simulation and temporal motion blur synthesis.
///
/// `frames` contains sequential temporal frames centered around the current frame.
/// `shutter_angle_deg`: 0.0 to 360.0 degrees (180.0 is traditional cinema shutter).
/// `samples`: number of sub-frame temporal samples to integrate.
pub fn apply_motion_blur_synthesis(
    frames: &[&[u8]],
    shutter_angle_deg: f64,
    samples: u32,
    width: usize,
    height: usize,
) -> Vec<u8> {
    let shutter_angle = shutter_angle_deg.clamp(0.0, 360.0);
    let pixel_count = width * height;
    let byte_count = pixel_count * 4;

    if frames.is_empty() {
        return vec![0u8; byte_count];
    }
    if frames.len() == 1 || shutter_angle <= 1.0 || samples <= 1 {
        let center = frames[frames.len() / 2];
        return center.to_vec();
    }

    let samples = samples.clamp(2, 32) as usize;
    let shutter_fraction = shutter_angle / 360.0; // 0.0 to 1.0 of the inter-frame period
    let center_idx = frames.len() / 2;

    // Accumulate weighted samples across the temporal exposure interval
    let mut accum_r = vec![0.0f32; pixel_count];
    let mut accum_g = vec![0.0f32; pixel_count];
    let mut accum_b = vec![0.0f32; pixel_count];
    let mut accum_a = vec![0.0f32; pixel_count];
    let mut total_weight = 0.0f32;

    for s in 0..samples {
        // Sample offset centered around middle frame: from -0.5*shutter to +0.5*shutter
        let sample_t = if samples > 1 {
            (s as f64 / (samples - 1) as f64 - 0.5) * shutter_fraction
        } else {
            0.0
        };

        // Gaussian weighting over shutter interval
        let weight = (-2.0 * sample_t * sample_t).exp() as f32;
        total_weight += weight;

        // Determine frame index and interpolation factor
        let float_idx = center_idx as f64 + sample_t * (frames.len() as f64 - 1.0);
        let f0 = (float_idx.floor() as usize).clamp(0, frames.len() - 1);
        let f1 = (float_idx.ceil() as usize).clamp(0, frames.len() - 1);
        let blend = (float_idx - float_idx.floor()) as f32;

        let frame0 = frames[f0];
        let frame1 = frames[f1];

        for i in 0..pixel_count {
            let idx = i * 4;
            let r = frame0[idx] as f32 * (1.0 - blend) + frame1[idx] as f32 * blend;
            let g = frame0[idx + 1] as f32 * (1.0 - blend) + frame1[idx + 1] as f32 * blend;
            let b = frame0[idx + 2] as f32 * (1.0 - blend) + frame1[idx + 2] as f32 * blend;
            let a = frame0[idx + 3] as f32 * (1.0 - blend) + frame1[idx + 3] as f32 * blend;

            accum_r[i] += r * weight;
            accum_g[i] += g * weight;
            accum_b[i] += b * weight;
            accum_a[i] += a * weight;
        }
    }

    let inv_weight = 1.0 / total_weight.max(1e-6);
    let mut out = vec![0u8; byte_count];
    for i in 0..pixel_count {
        let idx = i * 4;
        out[idx] = (accum_r[i] * inv_weight).round().clamp(0.0, 255.0) as u8;
        out[idx + 1] = (accum_g[i] * inv_weight).round().clamp(0.0, 255.0) as u8;
        out[idx + 2] = (accum_b[i] * inv_weight).round().clamp(0.0, 255.0) as u8;
        out[idx + 3] = (accum_a[i] * inv_weight).round().clamp(0.0, 255.0) as u8;
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use saturn_core::project::SpeedRampPoint;

    #[test]
    fn test_speed_ramp_interpolation() {
        let ramp = SpeedRamp {
            points: vec![
                SpeedRampPoint {
                    time: Tick(0),
                    speed_factor: 1.0,
                    bezier_tangent: (0.0, 0.0),
                },
                SpeedRampPoint {
                    time: Tick(100),
                    speed_factor: 0.25,
                    bezier_tangent: (0.0, 0.0),
                },
                SpeedRampPoint {
                    time: Tick(200),
                    speed_factor: 2.0,
                    bezier_tangent: (0.0, 0.0),
                },
            ],
            optical_flow_interpolation: true,
            frame_blend: false,
        };

        assert_eq!(interpolate_speed_ramp(&ramp, Tick(0)), 1.0);
        assert_eq!(interpolate_speed_ramp(&ramp, Tick(100)), 0.25);
        assert_eq!(interpolate_speed_ramp(&ramp, Tick(200)), 2.0);

        // Halfway between 0 and 100 should be between 0.25 and 1.0
        let mid = interpolate_speed_ramp(&ramp, Tick(50));
        assert!(mid > 0.25 && mid < 1.0);
    }

    #[test]
    fn test_optical_flow_frame_blending() {
        let width = 4;
        let height = 4;
        let mut prev = vec![0u8; 4 * 4 * 4];
        let mut next = vec![0u8; 4 * 4 * 4];

        // Fill with white vs black
        prev.fill(0);
        next.fill(200);

        let blended =
            blend_bidirectional_optical_flow(&prev, &next, None, None, 0.5, width, height);
        assert_eq!(blended.len(), 64);
        // At alpha=0.5, pixel value should be approximately 100
        assert!(blended[0] >= 90 && blended[0] <= 110);
    }

    #[test]
    fn test_dynamic_shutter_motion_blur() {
        let width = 2;
        let height = 2;
        let f0 = vec![100u8; 16];
        let f1 = vec![150u8; 16];
        let f2 = vec![200u8; 16];
        let frames: Vec<&[u8]> = vec![&f0, &f1, &f2];

        // 180 degree shutter angle with 8 samples
        let blurred = apply_motion_blur_synthesis(&frames, 180.0, 8, width, height);
        assert_eq!(blurred.len(), 16);
        // The center frame is 150, blurred across 100..200
        assert!(blurred[0] >= 140 && blurred[0] <= 160);
    }
}
