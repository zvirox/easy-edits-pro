use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct ShaderPreset {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub glsl_code: &'static str,
    pub default_uniforms: &'static [(&'static str, f64)],
}

pub static SHADER_PRESETS: &[ShaderPreset] = &[
    ShaderPreset {
        id: "crt_scanlines",
        name: "Retro CRT Scanlines",
        description: "Simulates vintage CRT cathode-ray scanlines and phosphor glow",
        glsl_code: r#"
precision highp float;
uniform sampler2D u_texture;
uniform vec2 u_resolution;
uniform float u_time;
uniform float u_scanline_density;
uniform float u_scanline_intensity;
varying vec2 v_texCoord;

void main() {
    vec4 color = texture2D(u_texture, v_texCoord);
    float scanline = sin(v_texCoord.y * u_resolution.y * u_scanline_density * 3.14159) * 0.5 + 0.5;
    color.rgb *= mix(1.0, scanline, u_scanline_intensity);
    gl_FragColor = color;
}
"#,
        default_uniforms: &[("u_scanline_density", 0.5), ("u_scanline_intensity", 0.35)],
    },
    ShaderPreset {
        id: "chromatic_aberration",
        name: "Chromatic Aberration",
        description: "Splits red and blue color channels radially for cinematic lens dispersion",
        glsl_code: r#"
precision highp float;
uniform sampler2D u_texture;
uniform vec2 u_resolution;
uniform float u_amount;
varying vec2 v_texCoord;

void main() {
    vec2 dir = v_texCoord - vec2(0.5);
    vec2 offset = dir * (u_amount * 0.05);
    float r = texture2D(u_texture, v_texCoord + offset).r;
    float g = texture2D(u_texture, v_texCoord).g;
    float b = texture2D(u_texture, v_texCoord - offset).b;
    float a = texture2D(u_texture, v_texCoord).a;
    gl_FragColor = vec4(r, g, b, a);
}
"#,
        default_uniforms: &[("u_amount", 0.5)],
    },
    ShaderPreset {
        id: "vignette",
        name: "Cinematic Vignette",
        description: "Smooth radial falloff darkening the edges of the frame",
        glsl_code: r#"
precision highp float;
uniform sampler2D u_texture;
uniform vec2 u_resolution;
uniform float u_radius;
uniform float u_softness;
varying vec2 v_texCoord;

void main() {
    vec4 color = texture2D(u_texture, v_texCoord);
    vec2 position = v_texCoord - vec2(0.5);
    float dist = length(position);
    float vignette = smoothstep(u_radius, u_radius - u_softness, dist);
    color.rgb *= vignette;
    gl_FragColor = color;
}
"#,
        default_uniforms: &[("u_radius", 0.75), ("u_softness", 0.45)],
    },
    ShaderPreset {
        id: "film_bloom",
        name: "Dreamy Film Bloom",
        description: "Soft romantic highlight diffusion and glow",
        glsl_code: r#"
precision highp float;
uniform sampler2D u_texture;
uniform vec2 u_resolution;
uniform float u_threshold;
uniform float u_intensity;
varying vec2 v_texCoord;

void main() {
    vec4 base = texture2D(u_texture, v_texCoord);
    float luma = dot(base.rgb, vec3(0.299, 0.587, 0.114));
    vec3 highlights = max(base.rgb - vec3(u_threshold), vec3(0.0));
    base.rgb += highlights * u_intensity;
    gl_FragColor = base;
}
"#,
        default_uniforms: &[("u_threshold", 0.65), ("u_intensity", 0.8)],
    },
    ShaderPreset {
        id: "invert",
        name: "Negative Invert",
        description: "Inverts RGB color values",
        glsl_code: r#"
precision highp float;
uniform sampler2D u_texture;
varying vec2 v_texCoord;

void main() {
    vec4 color = texture2D(u_texture, v_texCoord);
    gl_FragColor = vec4(1.0 - color.rgb, color.a);
}
"#,
        default_uniforms: &[],
    },
];

pub fn get_preset_by_id(id: &str) -> Option<&'static ShaderPreset> {
    SHADER_PRESETS.iter().find(|p| p.id == id)
}

/// Applies a shader preset via fast CPU rasterizer fallback for offline rendering / preview.
pub fn apply_shader_preset_buffer(
    rgba_buf: &mut [u8],
    width: usize,
    height: usize,
    preset_id: &str,
    uniforms: &HashMap<String, f64>,
    _time: f64,
) {
    match preset_id {
        "crt_scanlines" => {
            let density = uniforms.get("u_scanline_density").copied().unwrap_or(0.5);
            let intensity = uniforms
                .get("u_scanline_intensity")
                .copied()
                .unwrap_or(0.35) as f32;
            for y in 0..height {
                let norm_y = y as f32 / height as f32;
                let scanline =
                    (norm_y * height as f32 * density as f32 * std::f32::consts::PI).sin() * 0.5
                        + 0.5;
                let factor = 1.0 - (1.0 - scanline) * intensity;
                let row_start = y * width * 4;
                for x in 0..width {
                    let idx = row_start + x * 4;
                    rgba_buf[idx] = (rgba_buf[idx] as f32 * factor).round() as u8;
                    rgba_buf[idx + 1] = (rgba_buf[idx + 1] as f32 * factor).round() as u8;
                    rgba_buf[idx + 2] = (rgba_buf[idx + 2] as f32 * factor).round() as u8;
                }
            }
        }
        "chromatic_aberration" => {
            let amount = uniforms.get("u_amount").copied().unwrap_or(0.5) as f32;
            let orig = rgba_buf.to_vec();
            for y in 0..height {
                let ny = (y as f32 / height as f32) - 0.5;
                for x in 0..width {
                    let nx = (x as f32 / width as f32) - 0.5;
                    let off_x = (nx * amount * 15.0).round() as isize;
                    let off_y = (ny * amount * 15.0).round() as isize;

                    let rx = ((x as isize + off_x).clamp(0, width as isize - 1)) as usize;
                    let ry = ((y as isize + off_y).clamp(0, height as isize - 1)) as usize;
                    let bx = ((x as isize - off_x).clamp(0, width as isize - 1)) as usize;
                    let by = ((y as isize - off_y).clamp(0, height as isize - 1)) as usize;

                    let target_idx = (y * width + x) * 4;
                    let r_idx = (ry * width + rx) * 4;
                    let b_idx = (by * width + bx) * 4;

                    rgba_buf[target_idx] = orig[r_idx];
                    rgba_buf[target_idx + 2] = orig[b_idx + 2];
                }
            }
        }
        "vignette" => {
            let radius = uniforms.get("u_radius").copied().unwrap_or(0.75) as f32;
            let softness = uniforms.get("u_softness").copied().unwrap_or(0.45) as f32;
            for y in 0..height {
                let ny = (y as f32 / height as f32) - 0.5;
                for x in 0..width {
                    let nx = (x as f32 / width as f32) - 0.5;
                    let dist = (nx * nx + ny * ny).sqrt();
                    let factor = if dist <= radius - softness {
                        1.0
                    } else if dist >= radius {
                        0.0
                    } else {
                        1.0 - (dist - (radius - softness)) / softness.max(1e-4)
                    };
                    let idx = (y * width + x) * 4;
                    rgba_buf[idx] = (rgba_buf[idx] as f32 * factor).round() as u8;
                    rgba_buf[idx + 1] = (rgba_buf[idx + 1] as f32 * factor).round() as u8;
                    rgba_buf[idx + 2] = (rgba_buf[idx + 2] as f32 * factor).round() as u8;
                }
            }
        }
        "invert" => {
            for chunk in rgba_buf.chunks_exact_mut(4) {
                chunk[0] = 255 - chunk[0];
                chunk[1] = 255 - chunk[1];
                chunk[2] = 255 - chunk[2];
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_presets_exist() {
        assert!(get_preset_by_id("crt_scanlines").is_some());
        assert!(get_preset_by_id("chromatic_aberration").is_some());
        assert!(get_preset_by_id("invert").is_some());
    }

    #[test]
    fn test_invert_shader_buffer() {
        let mut buf = vec![10, 20, 30, 255];
        let uniforms = HashMap::new();
        apply_shader_preset_buffer(&mut buf, 1, 1, "invert", &uniforms, 0.0);
        assert_eq!(buf, vec![245, 235, 225, 255]);
    }
}
