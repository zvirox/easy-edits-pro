// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Pixel warping for planar corner pins (Packet 6.1).
//!
//! The corner quad is stored on the project document by the core command; this
//! module owns the inverse homography and bilinear resample. Motion blur is a
//! fixed three-tap blur along the quad's horizontal edge, deliberately not a
//! claim about measured camera motion.

use saturn_perception::motion_track::{apply_homography, homography_from_point_pairs};

/// Warp an RGBA frame into a four-corner quad. Corners are normalized
/// `[x, y]` in reading order (TL, TR, BR, BL) relative to the output frame.
pub fn warp_corner_pin_rgba(
    input: &[u8],
    width: usize,
    height: usize,
    corners: [[f64; 2]; 4],
    motion_blur: bool,
) -> Result<Vec<u8>, String> {
    if width == 0 || height == 0 {
        return Err("corner pin needs a non-empty frame".into());
    }
    let expected = width
        .checked_mul(height)
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or("corner pin frame size overflow")?;
    if input.len() != expected {
        return Err(format!(
            "corner pin expected {} RGBA bytes, got {}",
            expected,
            input.len()
        ));
    }

    let unit = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
    let pairs: Vec<([f64; 2], [f64; 2])> = corners
        .iter()
        .zip(unit.iter())
        .map(|(corner, source)| (*corner, *source))
        .collect();
    let inverse = homography_from_point_pairs(&pairs)
        .ok_or("corner pin quad cannot be solved as a homography")?;

    let sample = |u: f64, v: f64| -> [f32; 4] {
        let x = u * width as f64 - 0.5;
        let y = v * height as f64 - 0.5;
        let x0 = x.floor();
        let y0 = y.floor();
        let fx = x - x0;
        let fy = y - y0;
        let mut accum = [0.0f32; 4];
        for (dy, ox) in [(0i64, 0i64), (0, 1), (1, 0), (1, 1)] {
            let px = x0 as i64 + ox;
            let py = y0 as i64 + dy;
            if px < 0 || py < 0 || px >= width as i64 || py >= height as i64 {
                continue;
            }
            let weight = (if ox == 0 { 1.0 - fx } else { fx })
                * (if dy == 0 { 1.0 - fy } else { fy });
            let index = ((py as usize * width) + px as usize) * 4;
            for channel in 0..4 {
                accum[channel] += input[index + channel] as f32 * weight as f32;
            }
        }
        accum
    };

    // The blur direction follows the top edge of the quad in normalized space.
    let blur_u = (corners[1][0] - corners[0][0]) / width as f64;
    let blur_v = (corners[1][1] - corners[0][1]) / height as f64;
    let mut output = vec![0u8; expected];
    for py in 0..height {
        for px in 0..width {
            let destination = [(px as f64 + 0.5) / width as f64, (py as f64 + 0.5) / height as f64];
            let [u, v] = apply_homography(&inverse, destination);
            let taps: &[(f64, f64)] = if motion_blur {
                &[(-1.0, -1.0), (0.0, 0.0), (1.0, 1.0)]
            } else {
                &[(0.0, 0.0)]
            };
            let mut rgba = [0f32; 4];
            for (offset, _) in taps {
                let sample = sample(u + blur_u * *offset, v + blur_v * *offset);
                for channel in 0..4 {
                    rgba[channel] += sample[channel] / taps.len() as f32;
                }
            }
            let index = (py * width + px) * 4;
            for channel in 0..4 {
                output[index + channel] = rgba[channel].round().clamp(0.0, 255.0) as u8;
            }
        }
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn solid(width: usize, height: usize) -> Vec<u8> {
        let mut frame = vec![0u8; width * height * 4];
        for chunk in frame.chunks_exact_mut(4) {
            chunk.copy_from_slice(&[200, 100, 50, 255]);
        }
        frame
    }

    #[test]
    fn identity_quad_preserves_the_frame() {
        let frame = solid(4, 4);
        let identity = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
        let warped = warp_corner_pin_rgba(&frame, 4, 4, identity, false).expect("warp");
        assert_eq!(warped, frame);
    }

    #[test]
    fn rejects_a_wrong_buffer_size() {
        assert!(warp_corner_pin_rgba(&[0u8; 4], 2, 2, [[0.0; 2]; 4], false).is_err());
    }
}
