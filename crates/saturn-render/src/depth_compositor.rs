// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Depth-driven compositing (Packet 6.1).
//!
//! The depth grid is measured by `saturn-perception::depth_ort` (DA-V2 Small)
//! and handed in by the caller; this module owns only the pixel operations.
//! `DofBokeh` is a depth-weighted box blur, not a lens-simulation claim.

use serde::{Deserialize, Serialize};

use crate::compositor::{BlendMode, merge_alpha_over_rgba};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DepthCompositeMode {
    ZThreshold,
    DofBokeh,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub struct DepthCompositeOptions {
    pub mode: DepthCompositeMode,
    /// Relative depth split in 0..1 (0 = nearest, 1 = farthest).
    pub threshold: f32,
    /// Maximum blur radius in pixels for the depth-of-field mode.
    pub blur_radius: u32,
}

fn validate_buffers(
    foreground: &[u8],
    background: &[u8],
    depth: &[f32],
    width: usize,
    height: usize,
) -> Result<(), String> {
    let pixels = width
        .checked_mul(height)
        .ok_or("depth composite dimensions overflow")?;
    if foreground.len() != pixels * 4 || background.len() != pixels * 4 {
        return Err("depth composite needs two RGBA buffers of matching size".into());
    }
    if depth.len() != pixels {
        return Err("depth grid must hold one normalized value per pixel".into());
    }
    Ok(())
}

fn near_foreground_alpha(depth: &[f32], threshold: f32) -> Vec<f32> {
    let softness = 0.05f32;
    depth
        .iter()
        .map(|&value| {
            let low = threshold - softness;
            let high = threshold + softness;
            let ramp = ((value - low) / (high - low)).clamp(0.0, 1.0);
            1.0 - ramp * ramp * (3.0 - 2.0 * ramp)
        })
        .collect()
}

fn box_blur_rgba(input: &[u8], width: usize, height: usize, radius: usize) -> Vec<u8> {
    let mut output = vec![0u8; input.len()];
    for y in 0..height {
        for x in 0..width {
            let mut sums = [0u32; 4];
            let mut count = 0u32;
            for ny in y.saturating_sub(radius)..=(y + radius).min(height - 1) {
                for nx in x.saturating_sub(radius)..=(x + radius).min(width - 1) {
                    let index = (ny * width + nx) * 4;
                    for channel in 0..4 {
                        sums[channel] += input[index + channel] as u32;
                    }
                    count += 1;
                }
            }
            let index = (y * width + x) * 4;
            for channel in 0..4 {
                output[index + channel] = (sums[channel] / count) as u8;
            }
        }
    }
    output
}

fn radius_for_depth(depth: f32, max_radius: u32) -> usize {
    (depth.clamp(0.0, 1.0) * max_radius as f32).round() as usize
}

fn depth_weighted_blur_rgba(
    foreground: &[u8],
    depth: &[f32],
    width: usize,
    height: usize,
    max_radius: u32,
) -> Vec<u8> {
    let mut output = foreground.to_vec();
    for radius in 1..=max_radius.min(16) as usize {
        let targets: Vec<usize> = depth
            .iter()
            .enumerate()
            .filter(|(_, value)| radius_for_depth(**value, max_radius) == radius)
            .map(|(index, _)| index)
            .collect();
        if targets.is_empty() {
            continue;
        }
        let blurred = box_blur_rgba(foreground, width, height, radius);
        for pixel in targets {
            let index = pixel * 4;
            output[index..index + 4].copy_from_slice(&blurred[index..index + 4]);
        }
    }
    output
}

/// Composite a measured foreground over a background using relative depth.
pub fn composite_depth_rgba(
    foreground: &[u8],
    background: &[u8],
    depth: &[f32],
    width: usize,
    height: usize,
    options: DepthCompositeOptions,
) -> Result<Vec<u8>, String> {
    validate_buffers(foreground, background, depth, width, height)?;
    if !options.threshold.is_finite() || !(0.0..=1.0).contains(&options.threshold) {
        return Err("depth threshold must be finite and between 0 and 1".into());
    }
    match options.mode {
        DepthCompositeMode::ZThreshold => {
            let alpha = near_foreground_alpha(depth, options.threshold);
            let mut masked = foreground.to_vec();
            for (pixel, &coverage) in alpha.iter().enumerate() {
                masked[pixel * 4 + 3] = (masked[pixel * 4 + 3] as f32 * coverage).round() as u8;
            }
            let mut output = vec![0u8; masked.len()];
            merge_alpha_over_rgba(
                &masked,
                background,
                &mut output,
                width,
                height,
                1.0,
                BlendMode::Normal,
            );
            Ok(output)
        }
        DepthCompositeMode::DofBokeh => {
            let blurred = depth_weighted_blur_rgba(
                foreground,
                depth,
                width,
                height,
                options.blur_radius.min(16),
            );
            let mut output = vec![0u8; blurred.len()];
            merge_alpha_over_rgba(
                &blurred,
                background,
                &mut output,
                width,
                height,
                1.0,
                BlendMode::Normal,
            );
            Ok(output)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn solid(width: usize, height: usize, color: [u8; 4]) -> Vec<u8> {
        let mut frame = vec![0u8; width * height * 4];
        for chunk in frame.chunks_exact_mut(4) {
            chunk.copy_from_slice(&color);
        }
        frame
    }

    #[test]
    fn z_threshold_keeps_near_pixels_and_drops_far_pixels() {
        let foreground = solid(4, 1, [200, 10, 10, 255]);
        let background = solid(4, 1, [10, 10, 200, 255]);
        let depth = vec![0.0, 0.0, 1.0, 1.0];
        let output = composite_depth_rgba(
            &foreground,
            &background,
            &depth,
            4,
            1,
            DepthCompositeOptions {
                mode: DepthCompositeMode::ZThreshold,
                threshold: 0.5,
                blur_radius: 0,
            },
        )
        .expect("composite");
        assert_eq!(&output[0..4], &[200, 10, 10, 255]);
        assert_eq!(&output[12..16], &[10, 10, 200, 255]);
    }

    #[test]
    fn rejects_a_depth_grid_of_the_wrong_size() {
        let frame = solid(2, 2, [0, 0, 0, 255]);
        assert!(composite_depth_rgba(
            &frame,
            &frame,
            &[0.0],
            2,
            2,
            DepthCompositeOptions {
                mode: DepthCompositeMode::DofBokeh,
                threshold: 0.5,
                blur_radius: 2,
            },
        )
        .is_err());
    }
}
