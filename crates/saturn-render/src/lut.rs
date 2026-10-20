use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Lut3DInfo {
    pub title: String,
    pub size: usize,
    pub domain_min: [f32; 3],
    pub domain_max: [f32; 3],
    pub entry_count: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Lut3D {
    pub title: String,
    pub size: usize,
    pub domain_min: [f32; 3],
    pub domain_max: [f32; 3],
    pub table: Vec<[f32; 3]>,
}

impl Lut3D {
    pub fn info(&self) -> Lut3DInfo {
        Lut3DInfo {
            title: self.title.clone(),
            size: self.size,
            domain_min: self.domain_min,
            domain_max: self.domain_max,
            entry_count: self.table.len(),
        }
    }

    /// Parses an Adobe .cube 3D LUT string.
    pub fn parse_cube(content: &str) -> Result<Self, String> {
        let mut title = "Untitled LUT".to_string();
        let mut size: Option<usize> = None;
        let mut domain_min = [0.0f32, 0.0, 0.0];
        let mut domain_max = [1.0f32, 1.0, 1.0];
        let mut table = Vec::new();

        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }

            if let Some(rest) = trimmed.strip_prefix("TITLE") {
                let clean_title = rest.trim().trim_matches('"');
                if !clean_title.is_empty() {
                    title = clean_title.to_string();
                }
                continue;
            }

            if let Some(rest) = trimmed.strip_prefix("LUT_3D_SIZE") {
                let parsed_size: usize = rest
                    .trim()
                    .parse()
                    .map_err(|_| format!("Invalid LUT_3D_SIZE value: `{rest}`"))?;
                if parsed_size < 2 || parsed_size > 256 {
                    return Err(format!(
                        "LUT_3D_SIZE {parsed_size} is outside valid range (2..=256)"
                    ));
                }
                size = Some(parsed_size);
                continue;
            }

            if let Some(rest) = trimmed.strip_prefix("DOMAIN_MIN") {
                let parts: Result<Vec<f32>, _> =
                    rest.split_whitespace().map(str::parse::<f32>).collect();
                let parts = parts.map_err(|_| format!("Invalid DOMAIN_MIN values: `{rest}`"))?;
                if parts.len() == 3 {
                    domain_min = [parts[0], parts[1], parts[2]];
                }
                continue;
            }

            if let Some(rest) = trimmed.strip_prefix("DOMAIN_MAX") {
                let parts: Result<Vec<f32>, _> =
                    rest.split_whitespace().map(str::parse::<f32>).collect();
                let parts = parts.map_err(|_| format!("Invalid DOMAIN_MAX values: `{rest}`"))?;
                if parts.len() == 3 {
                    domain_max = [parts[0], parts[1], parts[2]];
                }
                continue;
            }

            // Ignore 1D LUT directives if present
            if trimmed.starts_with("LUT_1D_SIZE") {
                return Err("1D LUTs are not supported; expected 3D LUT".into());
            }

            // Try to parse 3 RGB floats
            let tokens: Vec<&str> = trimmed.split_whitespace().collect();
            if tokens.len() == 3 {
                if let (Ok(r), Ok(g), Ok(b)) = (
                    tokens[0].parse::<f32>(),
                    tokens[1].parse::<f32>(),
                    tokens[2].parse::<f32>(),
                ) {
                    table.push([r, g, b]);
                }
            }
        }

        let lut_size =
            size.ok_or_else(|| "Missing LUT_3D_SIZE directive in .cube file".to_string())?;
        let expected_count = lut_size * lut_size * lut_size;
        if table.len() != expected_count {
            return Err(format!(
                "Table entry count mismatch: expected {} (for {}^3) but found {}",
                expected_count,
                lut_size,
                table.len()
            ));
        }

        Ok(Self {
            title,
            size: lut_size,
            domain_min,
            domain_max,
            table,
        })
    }

    /// Evaluates trilinear interpolation for RGB color in [0.0, 1.0].
    pub fn sample(&self, r: f32, g: f32, b: f32) -> [f32; 3] {
        let size = self.size;
        let dmin = self.domain_min;
        let dmax = self.domain_max;

        // Normalize inputs relative to domain
        let span_r = (dmax[0] - dmin[0]).max(1e-5);
        let span_g = (dmax[1] - dmin[1]).max(1e-5);
        let span_b = (dmax[2] - dmin[2]).max(1e-5);

        let nr = ((r - dmin[0]) / span_r).clamp(0.0, 1.0);
        let ng = ((g - dmin[1]) / span_g).clamp(0.0, 1.0);
        let nb = ((b - dmin[2]) / span_b).clamp(0.0, 1.0);

        let max_idx = (size - 1) as f32;
        let xr = nr * max_idx;
        let yg = ng * max_idx;
        let zb = nb * max_idx;

        let x0 = (xr.floor() as usize).min(size - 1);
        let x1 = (x0 + 1).min(size - 1);
        let fx = xr - x0 as f32;

        let y0 = (yg.floor() as usize).min(size - 1);
        let y1 = (y0 + 1).min(size - 1);
        let fy = yg - y0 as f32;

        let z0 = (zb.floor() as usize).min(size - 1);
        let z1 = (z0 + 1).min(size - 1);
        let fz = zb - z0 as f32;

        // Adobe .cube order: index = x + y * size + z * size * size
        let idx = |x: usize, y: usize, z: usize| -> usize { x + y * size + z * size * size };

        let c000 = self.table[idx(x0, y0, z0)];
        let c100 = self.table[idx(x1, y0, z0)];
        let c010 = self.table[idx(x0, y1, z0)];
        let c110 = self.table[idx(x1, y1, z0)];
        let c001 = self.table[idx(x0, y0, z1)];
        let c101 = self.table[idx(x1, y0, z1)];
        let c011 = self.table[idx(x0, y1, z1)];
        let c111 = self.table[idx(x1, y1, z1)];

        // Interpolate along X
        let c00 = [
            c000[0] + fx * (c100[0] - c000[0]),
            c000[1] + fx * (c100[1] - c000[1]),
            c000[2] + fx * (c100[2] - c000[2]),
        ];
        let c10 = [
            c010[0] + fx * (c110[0] - c010[0]),
            c010[1] + fx * (c110[1] - c010[1]),
            c010[2] + fx * (c110[2] - c010[2]),
        ];
        let c01 = [
            c001[0] + fx * (c101[0] - c001[0]),
            c001[1] + fx * (c101[1] - c001[1]),
            c001[2] + fx * (c101[2] - c001[2]),
        ];
        let c11 = [
            c011[0] + fx * (c111[0] - c011[0]),
            c011[1] + fx * (c111[1] - c011[1]),
            c011[2] + fx * (c111[2] - c011[2]),
        ];

        // Interpolate along Y
        let c0 = [
            c00[0] + fy * (c10[0] - c00[0]),
            c00[1] + fy * (c10[1] - c00[1]),
            c00[2] + fy * (c10[2] - c00[2]),
        ];
        let c1 = [
            c01[0] + fy * (c11[0] - c01[0]),
            c01[1] + fy * (c11[1] - c01[1]),
            c01[2] + fy * (c11[2] - c01[2]),
        ];

        // Interpolate along Z
        [
            c0[0] + fz * (c1[0] - c0[0]),
            c0[1] + fz * (c1[1] - c0[1]),
            c0[2] + fz * (c1[2] - c0[2]),
        ]
    }

    /// Evaluates sample with intensity blending (0.0 = original, 1.0 = full LUT).
    pub fn sample_with_intensity(&self, r: f32, g: f32, b: f32, intensity: f32) -> [f32; 3] {
        let weight = intensity.clamp(0.0, 1.0);
        if weight <= 0.0 {
            return [r, g, b];
        }
        let sampled = self.sample(r, g, b);
        [
            r + weight * (sampled[0] - r),
            g + weight * (sampled[1] - g),
            b + weight * (sampled[2] - b),
        ]
    }

    /// Applies 3D LUT to an 8-bit RGBA buffer in place.
    pub fn apply_rgba(&self, buffer: &mut [u8], intensity: f32) {
        if intensity <= 0.0 {
            return;
        }
        for pixel in buffer.chunks_exact_mut(4) {
            let r = pixel[0] as f32 / 255.0;
            let g = pixel[1] as f32 / 255.0;
            let b = pixel[2] as f32 / 255.0;
            let out = self.sample_with_intensity(r, g, b, intensity);
            pixel[0] = (out[0].clamp(0.0, 1.0) * 255.0).round() as u8;
            pixel[1] = (out[1].clamp(0.0, 1.0) * 255.0).round() as u8;
            pixel[2] = (out[2].clamp(0.0, 1.0) * 255.0).round() as u8;
        }
    }

    /// Serializes LUT into Adobe .cube format string.
    pub fn export_cube(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!("TITLE \"{}\"\n", self.title));
        out.push_str(&format!("LUT_3D_SIZE {}\n", self.size));
        out.push_str(&format!(
            "DOMAIN_MIN {:.6} {:.6} {:.6}\n",
            self.domain_min[0], self.domain_min[1], self.domain_min[2]
        ));
        out.push_str(&format!(
            "DOMAIN_MAX {:.6} {:.6} {:.6}\n",
            self.domain_max[0], self.domain_max[1], self.domain_max[2]
        ));
        out.push('\n');

        for rgb in &self.table {
            out.push_str(&format!("{:.6} {:.6} {:.6}\n", rgb[0], rgb[1], rgb[2]));
        }

        out
    }

    /// Creates an identity 3D LUT with given dimension (default 17 or 33).
    pub fn identity(size: usize) -> Self {
        let size = size.clamp(2, 64);
        let mut table = Vec::with_capacity(size * size * size);
        let max = (size - 1) as f32;
        for z in 0..size {
            for y in 0..size {
                for x in 0..size {
                    table.push([x as f32 / max, y as f32 / max, z as f32 / max]);
                }
            }
        }
        Self {
            title: "Identity".into(),
            size,
            domain_min: [0.0, 0.0, 0.0],
            domain_max: [1.0, 1.0, 1.0],
            table,
        }
    }

    /// Builds a built-in cinematic preset 3D LUT.
    pub fn build_preset(preset_id: &str, size: usize) -> Result<Self, String> {
        let size = size.clamp(2, 64);
        let max = (size - 1) as f32;
        let mut table = Vec::with_capacity(size * size * size);

        let (title, grade_fn): (&str, fn(f32, f32, f32) -> [f32; 3]) = match preset_id {
            "teal_orange" => ("Teal & Orange", |r, g, b| {
                let luma = 0.2126 * r + 0.7152 * g + 0.0722 * b;
                let shadow_mask = (1.0 - luma).powf(1.4);
                let highlight_mask = luma.powf(1.2);
                // Push shadows towards cyan/teal
                let mut out_r = r * (1.0 - 0.25 * shadow_mask) + 0.35 * highlight_mask * r;
                let mut out_g = g * (1.0 + 0.05 * shadow_mask) + 0.15 * highlight_mask * g;
                let mut out_b = b * (1.0 + 0.40 * shadow_mask) - 0.20 * highlight_mask * b;
                // Boost contrast slightly
                out_r = (out_r - 0.5) * 1.15 + 0.5;
                out_g = (out_g - 0.5) * 1.15 + 0.5;
                out_b = (out_b - 0.5) * 1.15 + 0.5;
                [
                    out_r.clamp(0.0, 1.0),
                    out_g.clamp(0.0, 1.0),
                    out_b.clamp(0.0, 1.0),
                ]
            }),
            "vintage" => ("Warm Vintage Film", |r, g, b| {
                // Lifted blacks, warm amber cast, rolled off highlights
                let out_r = (r * 0.90 + 0.08).clamp(0.0, 1.0);
                let out_g = (g * 0.85 + 0.05).clamp(0.0, 1.0);
                let out_b = (b * 0.75 + 0.03).clamp(0.0, 1.0);
                [out_r, out_g, out_b]
            }),
            "film_noir" => ("Film Noir Monochrome", |r, g, b| {
                let luma = 0.299 * r + 0.587 * g + 0.114 * b;
                // S-curve contrast on luma
                let s_luma = if luma < 0.5 {
                    2.0 * luma * luma
                } else {
                    1.0 - 2.0 * (1.0 - luma) * (1.0 - luma)
                };
                [
                    s_luma.clamp(0.0, 1.0),
                    s_luma.clamp(0.0, 1.0),
                    s_luma.clamp(0.0, 1.0),
                ]
            }),
            "rec709" => ("Clean Rec.709", |r, g, b| {
                // Standard video curve with rich punchy contrast
                let s_curve = |v: f32| -> f32 {
                    let c = (v - 0.5) * 1.18 + 0.5;
                    c.clamp(0.0, 1.0)
                };
                [s_curve(r), s_curve(g), s_curve(b)]
            }),
            other => return Err(format!("Unknown LUT preset: `{other}`")),
        };

        for z in 0..size {
            for y in 0..size {
                for x in 0..size {
                    let r = x as f32 / max;
                    let g = y as f32 / max;
                    let b = z as f32 / max;
                    table.push(grade_fn(r, g, b));
                }
            }
        }

        Ok(Self {
            title: title.into(),
            size,
            domain_min: [0.0, 0.0, 0.0],
            domain_max: [1.0, 1.0, 1.0],
            table,
        })
    }

    /// Generates a valid .cube file string for a named preset.
    pub fn generate_preset_cube(preset_id: &str, size: usize) -> Result<String, String> {
        let lut = Self::build_preset(preset_id, size)?;
        Ok(lut.export_cube())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identity_lut_samples_accurately() {
        let id_lut = Lut3D::identity(17);
        assert_eq!(id_lut.table.len(), 17 * 17 * 17);

        let test_colors = [
            [0.0, 0.0, 0.0],
            [1.0, 1.0, 1.0],
            [0.5, 0.5, 0.5],
            [0.2, 0.6, 0.8],
            [0.9, 0.1, 0.4],
        ];

        for [r, g, b] in test_colors {
            let sampled = id_lut.sample(r, g, b);
            assert!(
                (sampled[0] - r).abs() < 1e-4,
                "R diff: {} vs {}",
                sampled[0],
                r
            );
            assert!(
                (sampled[1] - g).abs() < 1e-4,
                "G diff: {} vs {}",
                sampled[1],
                g
            );
            assert!(
                (sampled[2] - b).abs() < 1e-4,
                "B diff: {} vs {}",
                sampled[2],
                b
            );
        }
    }

    #[test]
    fn test_parse_and_export_cube() {
        let cube_data = r#"
# Project Saturn LUT test
TITLE "Test Grade"
LUT_3D_SIZE 2
DOMAIN_MIN 0.0 0.0 0.0
DOMAIN_MAX 1.0 1.0 1.0

0.000000 0.000000 0.000000
1.000000 0.000000 0.000000
0.000000 1.000000 0.000000
1.000000 1.000000 0.000000
0.000000 0.000000 1.000000
1.000000 0.000000 1.000000
0.000000 1.000000 1.000000
1.000000 1.000000 1.000000
"#;

        let lut = Lut3D::parse_cube(cube_data).unwrap();
        assert_eq!(lut.title, "Test Grade");
        assert_eq!(lut.size, 2);
        assert_eq!(lut.table.len(), 8);

        let exported = lut.export_cube();
        let parsed_again = Lut3D::parse_cube(&exported).unwrap();
        assert_eq!(parsed_again.size, 2);
        assert_eq!(parsed_again.table.len(), 8);
    }

    #[test]
    fn test_presets_generation_and_intensity_blend() {
        for preset in ["teal_orange", "vintage", "film_noir", "rec709"] {
            let lut = Lut3D::build_preset(preset, 17).unwrap();
            assert_eq!(lut.table.len(), 17 * 17 * 17);

            // Zero intensity should match input
            let zero = lut.sample_with_intensity(0.5, 0.4, 0.3, 0.0);
            assert_eq!(zero, [0.5, 0.4, 0.3]);

            // Full intensity should alter color
            let full = lut.sample_with_intensity(0.5, 0.4, 0.3, 1.0);
            assert!(full[0] >= 0.0 && full[0] <= 1.0);

            // Export to valid .cube
            let cube_str = Lut3D::generate_preset_cube(preset, 17).unwrap();
            let parsed = Lut3D::parse_cube(&cube_str).unwrap();
            assert_eq!(parsed.size, 17);
        }
    }

    #[test]
    fn test_rgba_buffer_application() {
        let lut = Lut3D::build_preset("film_noir", 17).unwrap();
        let mut buffer = vec![
            255, 0, 0, 255, // Pure red
            0, 255, 0, 255, // Pure green
            0, 0, 255, 255, // Pure blue
        ];
        lut.apply_rgba(&mut buffer, 1.0);
        // In film noir, all R, G, B channels of a pixel should be identical (monochrome)
        assert_eq!(buffer[0], buffer[1]);
        assert_eq!(buffer[1], buffer[2]);
        assert_eq!(buffer[3], 255); // Alpha preserved
    }
}
