// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Physical print emulation pre-passes (AI-native plan Packet 3.2):
//!
//! - [`gate_weave_offset`] — deterministic per-frame gate weave in UV units.
//!   Real gate weave is stochastic; this packet ships a documented
//!   deterministic model (incommensurate sinusoidal beat, amplitude
//!   proportional to the operator's weave setting). A measured statistical
//!   model is a later-packet item.
//! - [`HalationPipeline`] — bright-pass + blur producing the halation spill
//!   texture the colour-chain shader adds (scaled by the node strength).
//!   The Gaussian and the red-dominant spill tint are documented design
//!   constants, not measurements (§4.4).

use crate::color_nodes_gpu::ColorNodesError;
use crate::gpu_context::GpuContext;
use std::borrow::Cow;

/// Gate-weave amplitude expressed as a fraction of the 35 mm full-frame gate
/// width (36 mm across the image). `amplitude_uv = weave_mm / 36` is a
/// documented geometric definition, not a measurement.
pub fn weave_amplitude_uv(weave_mm: f64) -> f64 {
    weave_mm / 36.0
}

/// Deterministic gate-weave offset for `frame`, in clip-relative UV units
/// (fractions of frame). Incommensurate periods (24 and 17 frames) keep x and
/// y phase beats from aligning within a 408-frame window; amplitude is the
/// operator's `weave_mm` normalised by the full-frame gate width. Zero weave
/// yields an exactly steady frame.
pub fn gate_weave_offset(weave_mm: f64, frame: u32) -> [f64; 2] {
    if weave_mm <= 0.0 {
        return [0.0, 0.0];
    }
    let amplitude = weave_amplitude_uv(weave_mm);
    let t_x = f64::from(frame) * std::f64::consts::TAU / 24.0;
    let t_y = f64::from(frame) * std::f64::consts::TAU / 17.0;
    [
        amplitude * (t_x + 0.7).sin(),
        amplitude * (t_y + 0.3).cos(),
    ]
}

const HALATION_WGSL: &str = r#"
fn luma(c: vec3<f32>) -> f32 {
  return 0.2126 * c.r + 0.7152 * c.g + 0.0722 * c.b;
}

// Documented design constants (§4.4: not claimed as measurements):
// - highlight threshold at 1.0 scene-linear stop-1.0 white;
// - Gaussian sigma 2.0 px (2*sigma^2 = 8) over a 9x9 kernel;
// - red-dominant spill tint (1.0, 0.35, 0.15) for the emulsion scattering;
// - kernel normalisation 23.990704 (exact sum of the 81 kernel weights).
const THRESHOLD: f32 = 1.0;
const SIGMA2: f32 = 8.0;
const NORM: f32 = 23.990704;

@group(0) @binding(0) var src_tex: texture_2d<f32>;
@group(0) @binding(1) var out_tex: texture_storage_2d<rgba32float, write>;

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
  let dims = textureDimensions(src_tex);
  if (gid.x >= dims.x || gid.y >= dims.y) {
    return;
  }
  var acc = 0.0;
  for (var dy = -4; dy <= 4; dy = dy + 1) {
    for (var dx = -4; dx <= 4; dx = dx + 1) {
      let p = clamp(
        vec2<i32>(gid.xy) + vec2<i32>(dx, dy),
        vec2<i32>(0, 0),
        vec2<i32>(i32(dims.x) - 1, i32(dims.y) - 1)
      );
      let s = textureLoad(src_tex, vec2<u32>(p), 0).rgb;
      let bright = max(luma(s) - THRESHOLD, 0.0);
      let w = exp(-(f32(dx * dx + dy * dy)) / SIGMA2);
      acc = acc + w * bright;
    }
  }
  let spill = (acc / NORM) * vec3<f32>(1.0, 0.35, 0.15);
  textureStore(out_tex, gid.xy, vec4<f32>(spill, 1.0));
}
"#;

fn validate_wgsl(source: &str) -> Result<(), ColorNodesError> {
    let module = naga::front::wgsl::parse_str(source)
        .map_err(|e| ColorNodesError::WgslInvalid(e.emit_to_string(source)))?;
    let mut validator = naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::empty(),
    );
    validator
        .validate(&module)
        .map_err(|e| ColorNodesError::WgslInvalid(e.to_string()))?;
    Ok(())
}

/// Bright-pass + Gaussian blur producing the halation spill texture. Each
/// output pixel holds the red-dominant spill (unmultiplied by strength; the
/// chain shader scales it by the film node's halation strength).
pub struct HalationPipeline {
    pipeline: wgpu::ComputePipeline,
    bind_group_layout: wgpu::BindGroupLayout,
}

impl HalationPipeline {
    pub fn new(ctx: &GpuContext) -> Result<Self, ColorNodesError> {
        validate_wgsl(HALATION_WGSL)?;
        let guard = ctx.device().push_error_scope(wgpu::ErrorFilter::Validation);
        let module = ctx.device().create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("saturn-render halation pre-pass"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(HALATION_WGSL)),
        });
        let bind_group_layout =
            ctx.device()
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("saturn-render halation layout"),
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Texture {
                                sample_type: wgpu::TextureSampleType::Float { filterable: false },
                                view_dimension: wgpu::TextureViewDimension::D2,
                                multisampled: false,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::StorageTexture {
                                access: wgpu::StorageTextureAccess::WriteOnly,
                                format: wgpu::TextureFormat::Rgba32Float,
                                view_dimension: wgpu::TextureViewDimension::D2,
                            },
                            count: None,
                        },
                    ],
                });
        let pipeline_layout =
            ctx.device()
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("saturn-render halation pipeline layout"),
                    bind_group_layouts: &[Some(&bind_group_layout)],
                    immediate_size: 0,
                });
        let pipeline =
            ctx.device()
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("saturn-render halation pipeline"),
                    layout: Some(&pipeline_layout),
                    module: &module,
                    entry_point: Some("main"),
                    compilation_options: Default::default(),
                    cache: None,
                });
        ctx.device()
            .poll(wgpu::PollType::wait_indefinitely())
            .map_err(|e| ColorNodesError::Pipeline(e.to_string()))?;
        if let Some(error) = pollster::block_on(guard.pop()) {
            return Err(ColorNodesError::Pipeline(error.to_string()));
        }
        Ok(Self {
            pipeline,
            bind_group_layout,
        })
    }

    /// Computes the spill texture for `pixels` (RGBA f32, scene-linear).
    pub fn spill_texture(
        &self,
        ctx: &GpuContext,
        width: u32,
        height: u32,
        pixels: &[f32],
    ) -> Result<wgpu::Texture, ColorNodesError> {
        let input = ctx.create_rgba32float_texture(
            width,
            height,
            wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        );
        ctx.upload_rgba32float(&input, width, height, pixels);
        let output = ctx.create_rgba32float_texture(
            width,
            height,
            wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::COPY_SRC,
        );
        let guard = ctx.device().push_error_scope(wgpu::ErrorFilter::Validation);
        let input_view = input.create_view(&wgpu::TextureViewDescriptor::default());
        let output_view = output.create_view(&wgpu::TextureViewDescriptor::default());
        let bind_group = ctx.device().create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("saturn-render halation bind group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&input_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&output_view),
                },
            ],
        });
        let mut encoder = ctx
            .device()
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("saturn-render halation dispatch"),
            });
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("saturn-render halation pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups(width.div_ceil(8), height.div_ceil(8), 1);
        }
        ctx.queue().submit(Some(encoder.finish()));
        ctx.device()
            .poll(wgpu::PollType::wait_indefinitely())
            .map_err(|e| ColorNodesError::Dispatch(e.to_string()))?;
        if let Some(error) = pollster::block_on(guard.pop()) {
            return Err(ColorNodesError::Dispatch(error.to_string()));
        }
        Ok(output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gate_weave_is_deterministic_and_bounded() {
        for frame in 0..408u32 {
            let [dx, dy] = gate_weave_offset(0.5, frame);
            let amplitude = weave_amplitude_uv(0.5);
            assert!(dx.abs() <= amplitude + 1e-12, "frame {frame} dx {dx}");
            assert!(dy.abs() <= amplitude + 1e-12, "frame {frame} dy {dy}");
        }
        // Determinism: same frame, same offset.
        assert_eq!(gate_weave_offset(0.5, 17), gate_weave_offset(0.5, 17));
        // Zero weave must be an exactly steady frame.
        assert_eq!(gate_weave_offset(0.0, 100), [0.0, 0.0]);
        // The offsets actually move with the frame.
        assert_ne!(gate_weave_offset(0.5, 1), gate_weave_offset(0.5, 2));
    }

    #[test]
    fn weave_amplitude_is_fraction_of_the_36mm_gate() {
        assert!((weave_amplitude_uv(36.0) - 1.0).abs() < 1e-12);
        assert!((weave_amplitude_uv(0.036) - 0.001).abs() < 1e-12);
    }
}
