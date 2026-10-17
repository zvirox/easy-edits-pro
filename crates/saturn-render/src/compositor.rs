use saturn_core::project::{RotoSplineMask, RotoSplinePoint};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BlendMode {
    #[default]
    Normal,
    Multiply,
    Screen,
    Add,
    Overlay,
}

/// Porter-Duff Source-Over and standard blend modes for RGBA buffers.
///
/// Blends foreground over background into `out`.
/// All buffers must be of size `width * height * 4`.
pub fn merge_alpha_over_rgba(
    fg: &[u8],
    bg: &[u8],
    out: &mut [u8],
    width: usize,
    height: usize,
    opacity: f64,
    blend_mode: BlendMode,
) {
    let pixel_count = width * height;
    let opacity = opacity.clamp(0.0, 1.0) as f32;

    for i in 0..pixel_count {
        let idx = i * 4;
        let fg_r = fg[idx] as f32 / 255.0;
        let fg_g = fg[idx + 1] as f32 / 255.0;
        let fg_b = fg[idx + 2] as f32 / 255.0;
        let fg_a = (fg[idx + 3] as f32 / 255.0) * opacity;

        let bg_r = bg[idx] as f32 / 255.0;
        let bg_g = bg[idx + 1] as f32 / 255.0;
        let bg_b = bg[idx + 2] as f32 / 255.0;
        let bg_a = bg[idx + 3] as f32 / 255.0;

        if fg_a <= 0.0 {
            out[idx..idx + 4].copy_from_slice(&bg[idx..idx + 4]);
            continue;
        }

        // Apply blend mode to color channels
        let (blended_r, blended_g, blended_b) = match blend_mode {
            BlendMode::Normal => (fg_r, fg_g, fg_b),
            BlendMode::Multiply => (fg_r * bg_r, fg_g * bg_g, fg_b * bg_b),
            BlendMode::Screen => (
                1.0 - (1.0 - fg_r) * (1.0 - bg_r),
                1.0 - (1.0 - fg_g) * (1.0 - bg_g),
                1.0 - (1.0 - fg_b) * (1.0 - bg_b),
            ),
            BlendMode::Add => (
                (fg_r + bg_r).min(1.0),
                (fg_g + bg_g).min(1.0),
                (fg_b + bg_b).min(1.0),
            ),
            BlendMode::Overlay => (
                overlay_channel(fg_r, bg_r),
                overlay_channel(fg_g, bg_g),
                overlay_channel(fg_b, bg_b),
            ),
        };

        // Standard Porter-Duff Source-Over alpha composite
        let out_a = fg_a + bg_a * (1.0 - fg_a);
        if out_a > 0.0 {
            let out_r = (blended_r * fg_a + bg_r * bg_a * (1.0 - fg_a)) / out_a;
            let out_g = (blended_g * fg_a + bg_g * bg_a * (1.0 - fg_a)) / out_a;
            let out_b = (blended_b * fg_a + bg_b * bg_a * (1.0 - fg_a)) / out_a;

            out[idx] = (out_r.clamp(0.0, 1.0) * 255.0).round() as u8;
            out[idx + 1] = (out_g.clamp(0.0, 1.0) * 255.0).round() as u8;
            out[idx + 2] = (out_b.clamp(0.0, 1.0) * 255.0).round() as u8;
            out[idx + 3] = (out_a.clamp(0.0, 1.0) * 255.0).round() as u8;
        } else {
            out[idx] = 0;
            out[idx + 1] = 0;
            out[idx + 2] = 0;
            out[idx + 3] = 0;
        }
    }
}

fn overlay_channel(fg: f32, bg: f32) -> f32 {
    if bg < 0.5 {
        2.0 * fg * bg
    } else {
        1.0 - 2.0 * (1.0 - fg) * (1.0 - bg)
    }
}

/// Rasterizes a RotoSplineMask into an 8-bit grayscale alpha buffer (0..255).
/// Supports point-in-polygon ray casting, edge distance feathering, and invert toggle.
pub fn rasterize_roto_mask(
    mask: &RotoSplineMask,
    width: usize,
    height: usize,
) -> Vec<u8> {
    let mut alpha_map = vec![0u8; width * height];
    if mask.points.len() < 3 {
        if mask.inverted {
            alpha_map.fill(255);
        }
        return alpha_map;
    }

    // Tessellate / sample spline points if bezier handles exist
    let poly_pts = sample_spline_points(&mask.points, mask.closed);
    let n = poly_pts.len();
    if n < 3 {
        return alpha_map;
    }

    let feather = mask.feather.max(0.0) as f64;
    let opacity = mask.opacity.clamp(0.0, 1.0);

    for y in 0..height {
        let py = y as f64 + 0.5;
        for x in 0..width {
            let px = x as f64 + 0.5;

            // Point-in-polygon ray-casting test
            let inside = point_in_polygon(px, py, &poly_pts);

            // Compute minimum distance to polygon boundary
            let min_dist = min_dist_to_edges(px, py, &poly_pts, mask.closed);

            let factor = if feather <= 0.001 {
                if inside { 1.0 } else { 0.0 }
            } else {
                if inside {
                    // Inside: fully opaque once further inside than feather
                    if min_dist >= feather {
                        1.0
                    } else {
                        0.5 + 0.5 * (min_dist / feather)
                    }
                } else {
                    // Outside: fades out smoothly from edge
                    if min_dist >= feather {
                        0.0
                    } else {
                        0.5 - 0.5 * (min_dist / feather)
                    }
                }
            };

            let effective = if mask.inverted { 1.0 - factor } else { factor };
            let val = ((effective * opacity).clamp(0.0, 1.0) * 255.0).round() as u8;
            alpha_map[y * width + x] = val;
        }
    }

    alpha_map
}

fn sample_spline_points(points: &[RotoSplinePoint], closed: bool) -> Vec<(f64, f64)> {
    let mut sampled = Vec::new();
    let count = points.len();
    let num_segments = if closed { count } else { count.saturating_sub(1) };

    for i in 0..num_segments {
        let p0 = &points[i];
        let p1 = &points[(i + 1) % count];

        let has_handles = (p0.handle_out.0 != 0.0 || p0.handle_out.1 != 0.0)
            || (p1.handle_in.0 != 0.0 || p1.handle_in.1 != 0.0);

        if !has_handles {
            sampled.push((p0.x, p0.y));
        } else {
            // Cubic bezier subdivision (8 steps)
            let steps = 8;
            let c0 = (p0.x, p0.y);
            let c1 = (p0.x + p0.handle_out.0, p0.y + p0.handle_out.1);
            let c2 = (p1.x + p1.handle_in.0, p1.y + p1.handle_in.1);
            let c3 = (p1.x, p1.y);

            for s in 0..steps {
                let t = s as f64 / steps as f64;
                let omt = 1.0 - t;
                let bx = omt.powi(3) * c0.0 + 3.0 * omt.powi(2) * t * c1.0 + 3.0 * omt * t.powi(2) * c2.0 + t.powi(3) * c3.0;
                let by = omt.powi(3) * c0.1 + 3.0 * omt.powi(2) * t * c1.1 + 3.0 * omt * t.powi(2) * c2.1 + t.powi(3) * c3.1;
                sampled.push((bx, by));
            }
        }
    }
    sampled
}

fn point_in_polygon(px: f64, py: f64, poly: &[(f64, f64)]) -> bool {
    let mut inside = false;
    let n = poly.len();
    let mut j = n - 1;
    for i in 0..n {
        let (xi, yi) = poly[i];
        let (xj, yj) = poly[j];

        let intersect = ((yi > py) != (yj > py))
            && (px < (xj - xi) * (py - yi) / (yj - yi + 1e-12) + xi);
        if intersect {
            inside = !inside;
        }
        j = i;
    }
    inside
}

fn min_dist_to_edges(px: f64, py: f64, poly: &[(f64, f64)], closed: bool) -> f64 {
    let n = poly.len();
    if n == 0 {
        return f64::MAX;
    }
    let mut min_dist_sq = f64::MAX;
    let limit = if closed { n } else { n.saturating_sub(1) };

    for i in 0..limit {
        let (x1, y1) = poly[i];
        let (x2, y2) = poly[(i + 1) % n];

        let dx = x2 - x1;
        let dy = y2 - y1;
        let len_sq = dx * dx + dy * dy;

        let dist_sq = if len_sq <= 1e-12 {
            let dpx = px - x1;
            let dpy = py - y1;
            dpx * dpx + dpy * dpy
        } else {
            let t = (((px - x1) * dx + (py - y1) * dy) / len_sq).clamp(0.0, 1.0);
            let proj_x = x1 + t * dx;
            let proj_y = y1 + t * dy;
            let dpx = px - proj_x;
            let dpy = py - proj_y;
            dpx * dpx + dpy * dpy
        };

        if dist_sq < min_dist_sq {
            min_dist_sq = dist_sq;
        }
    }
    min_dist_sq.sqrt()
}

/// Particle system primitive for the Fusion Particles node.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Particle {
    pub x: f64,
    pub y: f64,
    pub vx: f64,
    pub vy: f64,
    pub age_sec: f64,
    pub lifetime_sec: f64,
    pub size: f64,
    pub color_rgba: [u8; 4],
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ParticleEmitter {
    pub origin_x: f64,
    pub origin_y: f64,
    pub initial_speed: f64,
    pub spread_rad: f64,
    pub base_angle_rad: f64,
    pub particle_lifetime_sec: f64,
    pub gravity_y: f64,
}

impl Default for ParticleEmitter {
    fn default() -> Self {
        Self {
            origin_x: 960.0,
            origin_y: 540.0,
            initial_speed: 120.0,
            spread_rad: std::f64::consts::PI * 2.0,
            base_angle_rad: -std::f64::consts::FRAC_PI_2,
            particle_lifetime_sec: 2.0,
            gravity_y: 50.0,
        }
    }
}

/// Simulates particle motion over `dt_sec`.
pub fn simulate_particle_field(
    particles: &mut [Particle],
    dt_sec: f64,
    emitter: &ParticleEmitter,
    seed: u64,
) {
    let mut rng_state = seed.wrapping_add(1337);

    for p in particles.iter_mut() {
        p.age_sec += dt_sec;
        if p.age_sec >= p.lifetime_sec {
            // Respawn particle at emitter
            p.age_sec = 0.0;
            p.x = emitter.origin_x;
            p.y = emitter.origin_y;

            // Simple pseudo-random angle and speed variation
            rng_state = rng_state.wrapping_mul(6364136223846793005).wrapping_add(1);
            let rand_val = (rng_state >> 32) as f64 / 4294967296.0;
            let angle = emitter.base_angle_rad + (rand_val - 0.5) * emitter.spread_rad;

            rng_state = rng_state.wrapping_mul(6364136223846793005).wrapping_add(1);
            let speed_var = 0.8 + 0.4 * ((rng_state >> 32) as f64 / 4294967296.0);
            let speed = emitter.initial_speed * speed_var;

            p.vx = angle.cos() * speed;
            p.vy = angle.sin() * speed;
        } else {
            // Apply physics step
            p.vy += emitter.gravity_y * dt_sec;
            p.x += p.vx * dt_sec;
            p.y += p.vy * dt_sec;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alpha_over_normal_blend() {
        let width = 2;
        let height = 2;
        // 50% transparent pure red over opaque pure blue
        let fg = vec![255, 0, 0, 128, 255, 0, 0, 128, 255, 0, 0, 128, 255, 0, 0, 128];
        let bg = vec![0, 0, 255, 255, 0, 0, 255, 255, 0, 0, 255, 255, 0, 0, 255, 255];
        let mut out = vec![0u8; 16];

        merge_alpha_over_rgba(&fg, &bg, &mut out, width, height, 1.0, BlendMode::Normal);

        // fg_a is ~0.502, bg_a is 1.0 -> out_a is 1.0
        // red channel: ~128, blue channel: ~127
        assert_eq!(out[3], 255);
        assert!(out[0] > 120 && out[0] < 135);
        assert!(out[2] > 120 && out[2] < 135);
    }

    #[test]
    fn test_roto_mask_rasterization_and_feather() {
        let width = 100;
        let height = 100;
        let mask = RotoSplineMask {
            id: 1,
            name: "Center Square".into(),
            closed: true,
            inverted: false,
            feather: 5.0,
            opacity: 1.0,
            points: vec![
                RotoSplinePoint { x: 20.0, y: 20.0, handle_in: (0.0, 0.0), handle_out: (0.0, 0.0) },
                RotoSplinePoint { x: 80.0, y: 20.0, handle_in: (0.0, 0.0), handle_out: (0.0, 0.0) },
                RotoSplinePoint { x: 80.0, y: 80.0, handle_in: (0.0, 0.0), handle_out: (0.0, 0.0) },
                RotoSplinePoint { x: 20.0, y: 80.0, handle_in: (0.0, 0.0), handle_out: (0.0, 0.0) },
            ],
        };

        let mask_buf = rasterize_roto_mask(&mask, width, height);
        assert_eq!(mask_buf.len(), 10000);

        // Center pixel (50, 50) is deep inside -> 255
        assert_eq!(mask_buf[50 * 100 + 50], 255);
        // Corner pixel (5, 5) is far outside -> 0
        assert_eq!(mask_buf[5 * 100 + 5], 0);
        // Pixel near boundary (20, 50) has feather transition
        let edge_val = mask_buf[50 * 100 + 20];
        assert!(edge_val > 50 && edge_val < 200);
    }

    #[test]
    fn test_particle_field_simulation_step() {
        let emitter = ParticleEmitter {
            origin_x: 100.0,
            origin_y: 100.0,
            initial_speed: 50.0,
            spread_rad: 0.0, // shoots directly upwards
            base_angle_rad: -std::f64::consts::FRAC_PI_2,
            particle_lifetime_sec: 1.0,
            gravity_y: 20.0,
        };

        let mut particles = vec![Particle {
            x: 100.0,
            y: 100.0,
            vx: 0.0,
            vy: -50.0,
            age_sec: 0.0,
            lifetime_sec: 1.0,
            size: 2.0,
            color_rgba: [255, 255, 255, 255],
        }];

        // Advance 0.1s
        simulate_particle_field(&mut particles, 0.1, &emitter, 42);
        assert!(particles[0].y < 100.0); // Moved up
        assert_eq!(particles[0].age_sec, 0.1);
    }
}
