// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Optical-flow clean-plate inpainting (Packet 6.1).
//!
//! The target frame, chronological donor frames and their measured
//! displacements come from the caller (decoded via `saturn-render` and tracked
//! via `saturn-perception`). This module only reconstructs pixels inside the
//! removal region. There is no learned/generative fill and no fake success:
//! when no donor covers a pixel, the nearest clean border pixel is used.

fn region_bounds(region: [f64; 4], width: usize, height: usize) -> Result<[usize; 4], String> {
    let [left, top, region_width, region_height] = region;
    if !region.iter().all(|value| value.is_finite())
        || !(0.0..1.0).contains(&left)
        || !(0.0..1.0).contains(&top)
        || region_width <= 0.0
        || region_height <= 0.0
        || left + region_width > 1.0
        || top + region_height > 1.0
    {
        return Err("removal region must be a finite normalized rectangle inside the frame".into());
    }
    let x0 = (left * width as f64) as usize;
    let y0 = (top * height as f64) as usize;
    let x1 = (((left + region_width) * width as f64) as usize).min(width);
    let y1 = (((top + region_height) * height as f64) as usize).min(height);
    Ok([x0, y0, x1.max(x0 + 1), y1.max(y0 + 1)])
}

fn donor_pixel(
    donors: &[&[u8]],
    displacements: &[(f32, f32)],
    width: usize,
    height: usize,
    x: usize,
    y: usize,
) -> Option<[u8; 4]> {
    for (donor, displacement) in donors.iter().zip(displacements) {
        let source_x = x as i64 + displacement.0.round() as i64;
        let source_y = y as i64 + displacement.1.round() as i64;
        if source_x < 0 || source_y < 0 || source_x >= width as i64 || source_y >= height as i64 {
            continue;
        }
        let index = (source_y as usize * width + source_x as usize) * 4;
        return Some([
            donor[index],
            donor[index + 1],
            donor[index + 2],
            donor[index + 3],
        ]);
    }
    None
}

fn nearest_border_pixel(frame: &[u8], bounds: [usize; 4], width: usize, x: usize, y: usize) -> [u8; 4] {
    let [x0, y0, x1, y1] = bounds;
    let sample_x = if x.saturating_sub(x0) < x1.saturating_sub(x + 1) {
        x0.saturating_sub(1)
    } else {
        x1.min(width - 1)
    };
    let sample_y = if y.saturating_sub(y0) < y1.saturating_sub(y + 1) {
        y0.saturating_sub(1)
    } else {
        y1
    };
    let index = (sample_y.min(height_minus_one(frame, width)) * width + sample_x) * 4;
    [
        frame[index],
        frame[index + 1],
        frame[index + 2],
        frame[index + 3],
    ]
}

fn height_minus_one(frame: &[u8], width: usize) -> usize {
    frame.len() / width / 4 - 1
}

/// Replace the removal region in `target` with tracked donor pixels.
pub fn inpaint_region_rgba(
    target: &[u8],
    donors: &[&[u8]],
    displacements: &[(f32, f32)],
    region: [f64; 4],
    width: usize,
    height: usize,
) -> Result<Vec<u8>, String> {
    if width == 0 || height == 0 {
        return Err("inpainting needs a non-empty frame".into());
    }
    let expected = width
        .checked_mul(height)
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or("inpainting frame size overflow")?;
    if target.len() != expected || donors.iter().any(|donor| donor.len() != expected) {
        return Err("inpainting needs RGBA buffers of matching size".into());
    }
    if donors.len() != displacements.len() || donors.len() > 8 {
        return Err("inpainting needs one displacement per donor, at most eight".into());
    }
    let bounds = region_bounds(region, width, height)?;
    let [x0, y0, x1, y1] = bounds;
    let mut output = target.to_vec();
    for y in y0..y1 {
        for x in x0..x1 {
            let pixel = donor_pixel(donors, displacements, width, height, x, y)
                .unwrap_or_else(|| nearest_border_pixel(target, bounds, width, x, y));
            let index = (y * width + x) * 4;
            output[index..index + 4].copy_from_slice(&pixel);
        }
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(width: usize, height: usize, color: [u8; 4]) -> Vec<u8> {
        let mut buffer = vec![0u8; width * height * 4];
        for pixel in buffer.chunks_exact_mut(4) {
            pixel.copy_from_slice(&color);
        }
        buffer
    }

    #[test]
    fn donor_pixels_replace_the_removal_region() {
        let target = frame(4, 4, [200, 10, 10, 255]);
        let donor = frame(4, 4, [10, 200, 10, 255]);
        let output = inpaint_region_rgba(
            &target,
            &[&donor],
            &[(0.0, 0.0)],
            [0.25, 0.25, 0.5, 0.5],
            4,
            4,
        )
        .expect("inpaint");
        assert_eq!(&output[(1 * 4 + 1) * 4..(1 * 4 + 1) * 4 + 4], &[10, 200, 10, 255]);
        assert_eq!(&output[0..4], &[200, 10, 10, 255]);
    }

    #[test]
    fn rejects_mismatched_displacements() {
        let frame = frame(2, 2, [0, 0, 0, 255]);
        assert!(inpaint_region_rgba(
            &frame,
            &[&frame],
            &[],
            [0.0, 0.0, 1.0, 1.0],
            2,
            2,
        )
        .is_err());
    }
}
