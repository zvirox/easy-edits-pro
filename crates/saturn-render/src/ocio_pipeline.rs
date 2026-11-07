// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! WGSL colour-management pipeline (Packet 3.1).
//!
//! Despite the module name (kept for API continuity per the plan-review
//! revision §4.6), **no OpenColorIO C++ dependency is linked**. The transform
//! set is the fixed, small WGSL program generated below from the Rust
//! constants in [`crate::ocio_transforms`] — those constants are the single
//! source of truth, so the CPU reference tests directly constrain the GPU
//! math.
//!
//! Per-pixel program shape:
//! `code value → IDT curve → camera linear → AP0 (ACES2065-1) → working
//! space (ACEScg linear or ACEScct log) → AP0 → XYZ-D65 (ASWF fold) →
//! display RGB → display transfer (sRGB piecewise or PQ)`.
//!
//! Determinism: float operation order is fixed in the WGSL text and mirrors
//! the CPU reference (`mat3_apply` row-dot order); matrices are uploaded as
//! row `vec4`s to avoid any column-major transpose ambiguity. The WGSL is
//! compile-time text; naga validation runs during pipeline construction and
//! the test-suite exercises it on every gate run (§10.2.3 hardening for
//! user-authored shaders belongs to Packet 3.2).

use std::borrow::Cow;
use std::num::NonZeroU64;

use crate::gpu_context::{GpuContext, GpuContextError};
use saturn_core::color::transforms::{
    self as ocio_transforms, mat3_apply, DisplayOdt, Mat3, WorkingSpace, APPLE_LOG_BETA,
    APPLE_LOG_DELTA, APPLE_LOG_GAMMA, APPLE_LOG_R0, APPLE_LOG_RT, APPLE_LOG_SIGMA, BMDFILM_GEN5_A,
    BMDFILM_GEN5_B, BMDFILM_GEN5_C, BMDFILM_GEN5_D, BMDFILM_GEN5_E, BMDFILM_GEN5_LIN_CUT,
    LOG3G10_A, LOG3G10_B, LOG3G10_C, LOG3G10_G, M_APPLE_WIDE_GAMUT_TO_AP0, M_AP0_TO_XYZ_D65,
    M_AWG4_TO_AP0, M_BMD_WIDE_GAMUT_TO_AP0, M_RWG_TO_AP0, M_SGAMUT3_CINE_TO_AP0,
};

/// Selection of transforms for one pipeline invocation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OcioConfig {
    /// Camera Input Device Transform applied to the input code values.
    pub idt: ocio_transforms::CameraIdt,
    /// Scene-referred working space.
    pub working: WorkingSpace,
    /// Display output transform.
    pub odt: DisplayOdt,
}

impl OcioConfig {
    /// Default editing configuration: video-level input, ACEScg working
    /// space, Rec.709 display.
    pub fn default_editing() -> Self {
        Self {
            idt: ocio_transforms::CameraIdt::Rec709Video,
            working: WorkingSpace::AcesCg,
            odt: DisplayOdt::Rec709,
        }
    }

    fn idt_kind(self) -> u32 {
        match self.idt {
            ocio_transforms::CameraIdt::Rec709Video => 0,
            ocio_transforms::CameraIdt::ArriLogC4 => 1,
            ocio_transforms::CameraIdt::RedLog3G10 => 2,
            ocio_transforms::CameraIdt::SonySLog3 => 3,
            ocio_transforms::CameraIdt::BlackmagicGen5 => 4,
            ocio_transforms::CameraIdt::AppleLog => 5,
        }
    }

    fn working_kind(self) -> u32 {
        match self.working {
            WorkingSpace::AcesCg => 0,
            WorkingSpace::AcesCct => 1,
        }
    }

    fn odt_kind(self) -> u32 {
        match self.odt {
            DisplayOdt::Rec709 => 0,
            DisplayOdt::DisplayP3 => 1,
            DisplayOdt::Rec2020Pq => 2,
        }
    }
}

/// Packs a row-major 3×3 matrix into three 16-byte rows for the uniform.
fn matrix_rows(m: &Mat3) -> [[f32; 4]; 3] {
    [
        [m[0][0] as f32, m[0][1] as f32, m[0][2] as f32, 0.0],
        [m[1][0] as f32, m[1][1] as f32, m[1][2] as f32, 0.0],
        [m[2][0] as f32, m[2][1] as f32, m[2][2] as f32, 0.0],
    ]
}

fn identity3() -> Mat3 {
    [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]
}

/// Camera-gamut → AP0 matrix for the given IDT (exact vendor matrices where
/// published; primaries-derived with Bradford adaptation for Rec.709 video).
fn idt_matrix(idt: ocio_transforms::CameraIdt) -> Mat3 {
    match idt {
        ocio_transforms::CameraIdt::Rec709Video => ocio_transforms::rec709_to_ap0_matrix(),
        ocio_transforms::CameraIdt::ArriLogC4 => M_AWG4_TO_AP0,
        ocio_transforms::CameraIdt::RedLog3G10 => M_RWG_TO_AP0,
        ocio_transforms::CameraIdt::SonySLog3 => M_SGAMUT3_CINE_TO_AP0,
        ocio_transforms::CameraIdt::BlackmagicGen5 => M_BMD_WIDE_GAMUT_TO_AP0,
        ocio_transforms::CameraIdt::AppleLog => M_APPLE_WIDE_GAMUT_TO_AP0,
    }
}

/// XYZ-D65 → display-RGB linear matrix for the given ODT.
fn display_matrix(odt: DisplayOdt) -> Mat3 {
    match odt {
        DisplayOdt::Rec709 => ocio_transforms::rec709_from_xyz_d65(),
        DisplayOdt::DisplayP3 => ocio_transforms::display_p3_from_xyz_d65(),
        DisplayOdt::Rec2020Pq => ocio_transforms::rec2020_from_xyz_d65(),
    }
}

/// CPU reference of the full GPU program (bit-accuracy baseline for tests
/// and a CPU path available to callers): code value → display-encoded.
pub fn transform_reference(cv: [f64; 3], config: OcioConfig) -> [f64; 3] {
    let ap0 = config.idt.decode_to_ap0(cv);
    let working = config.working.from_ap0(ap0);
    let graded = config.working.to_ap0(working);
    let xyz = mat3_apply(&M_AP0_TO_XYZ_D65, graded);
    let display_linear = mat3_apply(&display_matrix(config.odt), xyz);
    match config.odt {
        DisplayOdt::Rec709 | DisplayOdt::DisplayP3 => display_linear.map(ocio_transforms::srgb_encode),
        DisplayOdt::Rec2020Pq => display_linear.map(|y| ocio_transforms::pq_encode(y / 100.0)),
    }
}

/// Uniform buffer payload (272 bytes = 68 u32/f32 words): kinds + dims
/// header (8 words / 32 B) plus five row-packed 3×3 matrices (idt, odt XYZ
/// fold, display, AP0↔AP1 working encode/decode) — matches the WGSL
/// `OcioParams` struct size exactly. Kind/dim fields are u32; matrices are
/// f32 bit patterns.
fn pack_params(config: OcioConfig, width: u32, height: u32) -> [u32; 68] {
    let idt_m = matrix_rows(&idt_matrix(config.idt));
    let odt_m = matrix_rows(&M_AP0_TO_XYZ_D65);
    let display_m = matrix_rows(&display_matrix(config.odt));
    let (cg_enc, cg_dec) = match config.working {
        WorkingSpace::AcesCg => (
            matrix_rows(&ocio_transforms::M_AP0_TO_AP1),
            matrix_rows(&ocio_transforms::M_AP1_TO_AP0),
        ),
        WorkingSpace::AcesCct => (matrix_rows(&identity3()), matrix_rows(&identity3())),
    };
    let mut words = [0u32; 68];
    words[0] = config.idt_kind();
    words[1] = config.working_kind();
    words[2] = config.odt_kind();
    words[4] = width;
    words[5] = height;
    let mut cursor = 8usize;
    for block in [idt_m, odt_m, display_m, cg_enc, cg_dec] {
        for row in block {
            for (i, v) in row.iter().enumerate() {
                words[cursor + i] = v.to_bits();
            }
            cursor += 4;
        }
    }
    debug_assert_eq!(cursor, 68);
    words
}

fn f64_fmt(v: f64) -> String {
    // Plain decimal fixed-point, enough digits to round-trip f32 casting.
    format!("{v:.10}")
}

/// WGSL program text, generated from the Rust curve constants so the CPU
/// reference tests govern the GPU math at the constant level.
pub fn wgsl_source() -> String {
    let (logc4_a, logc4_s, logc4_t) = ocio_transforms::logc4_consts();
    format!(
        r#"// Generated by saturn-render ocio_pipeline::wgsl_source — do not edit.
// Curve constants are emitted from crates/saturn-render/src/ocio_transforms.rs.

struct OcioParams {{
    idt_kind: u32,
    working_kind: u32,
    odt_kind: u32,
    _pad: u32,
    dims: vec2<u32>,
    _pad2: vec2<u32>,
    idt_matrix: array<vec4<f32>, 3>,
    odt_matrix: array<vec4<f32>, 3>,
    display_matrix: array<vec4<f32>, 3>,
    cg_encode: array<vec4<f32>, 3>,
    cg_decode: array<vec4<f32>, 3>,
}}

@group(0) @binding(0) var input_tex: texture_2d<f32>;
@group(0) @binding(1) var output_tex: texture_storage_2d<rgba32float, write>;
@group(0) @binding(2) var<uniform> p: OcioParams;

// Row-major apply matching the CPU reference order exactly:
// out[i] = m[i][0]*v.x + m[i][1]*v.y + m[i][2]*v.z
fn apply3(m: array<vec4<f32>, 3>, v: vec3<f32>) -> vec3<f32> {{
    return vec3<f32>(
        (m[0].x * v.x) + (m[0].y * v.y) + (m[0].z * v.z),
        (m[1].x * v.x) + (m[1].y * v.y) + (m[1].z * v.z),
        (m[2].x * v.x) + (m[2].y * v.y) + (m[2].z * v.z),
    );
}}

const LOGC4_A: f32 = {logc4_a};
const LOGC4_S: f32 = {logc4_s};
const LOGC4_T: f32 = {logc4_t};
const LOGC4_B: f32 = {logc4_b}f;
const LOGC4_C: f32 = {logc4_c}f;
const LOG3G10_A: f32 = {log3g10_a}f;
const LOG3G10_B: f32 = {log3g10_b}f;
const LOG3G10_C: f32 = {log3g10_c}f;
const LOG3G10_G: f32 = {log3g10_g}f;
const BMD_A: f32 = {bmd_a}f;
const BMD_B: f32 = {bmd_b}f;
const BMD_C: f32 = {bmd_c}f;
const BMD_D: f32 = {bmd_d}f;
const BMD_E: f32 = {bmd_e}f;
const BMD_LIN_CUT: f32 = {bmd_lin_cut}f;
const APPLE_R0: f32 = {apple_r0}f;
const APPLE_RT: f32 = {apple_rt}f;
const APPLE_SIGMA: f32 = {apple_sigma}f;
const APPLE_BETA: f32 = {apple_beta}f;
const APPLE_GAMMA: f32 = {apple_gamma}f;
const APPLE_DELTA: f32 = {apple_delta}f;
const APPLE_SCENE_SCALE: f32 = 0.9f;
const ACESCCT_CUT: f32 = {acescct_cut}f;
const ACESCCT_A: f32 = {acescct_a}f;
const ACESCCT_B: f32 = {acescct_b}f;
const PQ_M1: f32 = {pq_m1}f;
const PQ_M2: f32 = {pq_m2}f;
const PQ_C1: f32 = {pq_c1}f;
const PQ_C2: f32 = {pq_c2}f;
const PQ_C3: f32 = {pq_c3}f;
const SRGB_CUT: f32 = 0.0031308f;

fn logc4_decode(cv: f32) -> f32 {{
    if (cv < 0.0f) {{
        return (cv * LOGC4_S) + LOGC4_T;
    }}
    let pw = ((14.0f * (cv - LOGC4_C)) / LOGC4_B) + 6.0f;
    return (exp2(pw) - 64.0f) / LOGC4_A;
}}

fn log3g10_decode(cv: f32) -> f32 {{
    if (cv < 0.0f) {{
        return (cv / LOG3G10_G) - LOG3G10_C;
    }}
    let z = exp2((log2(10.0f) * cv) / LOG3G10_A);
    return max(((z - 1.0f) / LOG3G10_B) - LOG3G10_C, -LOG3G10_C);
}}

fn slog3_decode(cv: f32) -> f32 {{
    var refl: f32;
    if (cv >= 420.0f / 1023.0f) {{
        refl = (0.19f * exp2((log2(10.0f) * ((cv * 1023.0f) - 420.0f)) / 261.5f)) - 0.01f;
    }} else {{
        refl = ((cv * 1023.0f) - 95.0f) * 0.01125f / (171.2102946929f - 95.0f);
    }}
    return refl * 0.9f;
}}

fn bmd_gen5_decode(cv: f32) -> f32 {{
    let log_cut = (BMD_D * BMD_LIN_CUT) + BMD_E;
    if (cv < log_cut) {{
        return (cv - BMD_E) / BMD_D;
    }}
    return exp((cv - BMD_C) / BMD_A) - BMD_B;
}}

fn apple_log_decode(cv: f32) -> f32 {{
    let p_t = APPLE_SIGMA * (APPLE_RT - APPLE_R0) * (APPLE_RT - APPLE_R0);
    if (cv >= p_t) {{
        return exp2((cv - APPLE_DELTA) / APPLE_GAMMA) - APPLE_BETA;
    }}
    if (cv >= 0.0f) {{
        return sqrt(cv / APPLE_SIGMA) + APPLE_R0;
    }}
    return 0.0f;
}}

fn rec709_oetf_decode(v: f32) -> f32 {{
    if (v < 0.081f) {{
        return v / 4.5f;
    }}
    return pow((v + 0.099f) / 1.099f, 1.0f / 0.45f);
}}

fn idt_decode(cv: f32) -> f32 {{
    switch (p.idt_kind) {{
        case 0u: {{ return rec709_oetf_decode(cv); }} // rec709_video
        case 1u: {{ return logc4_decode(cv); }} // arri_logc4
        case 2u: {{ return log3g10_decode(cv); }} // red_log3g10
        case 3u: {{ return slog3_decode(cv); }} // sony_slog3
        case 4u: {{ return bmd_gen5_decode(cv); }} // blackmagic_gen5
        default: {{ return apple_log_decode(cv) * APPLE_SCENE_SCALE; }} // apple_log
    }}
}}

fn acescct_encode(linear: f32) -> f32 {{
    if (linear <= ACESCCT_CUT) {{
        return (ACESCCT_A * linear) + ACESCCT_B;
    }}
    return (log2(linear) + 9.72f) / 17.52f;
}}

fn acescct_decode(cv: f32) -> f32 {{
    if (cv <= 0.155251141552511f) {{
        return (cv - ACESCCT_B) / ACESCCT_A;
    }}
    return exp2((17.52f * cv) - 9.72f);
}}

fn srgb_encode(linear: f32) -> f32 {{
    if (linear <= SRGB_CUT) {{
        return 12.92f * linear;
    }}
    return (1.055f * pow(linear, 1.0f / 2.4f)) - 0.055f;
}}

fn pq_encode(y_relative: f32) -> f32 {{
    if (y_relative <= 0.0f) {{
        return 0.0f;
    }}
    let yp = pow(y_relative, PQ_M1);
    return pow((PQ_C1 + (PQ_C2 * yp)) / (1.0f + (PQ_C3 * yp)), PQ_M2);
}}

fn display_encode(linear: f32) -> f32 {{
    switch (p.odt_kind) {{
        case 0u, 1u: {{ return srgb_encode(linear); }} // rec709, display_p3
        default: {{ return pq_encode(linear / 100.0f); }} // rec2020_pq, SDR white = 100 nits
    }}
}}

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {{
    if ((gid.x >= p.dims.x) || (gid.y >= p.dims.y)) {{
        return;
    }}
    let cv = textureLoad(input_tex, gid.xy, 0).rgb;
    let linear = vec3<f32>(idt_decode(cv.r), idt_decode(cv.g), idt_decode(cv.b));
    let ap0 = apply3(p.idt_matrix, linear);
    // Working space: ACEScg is AP1 linear; ACEScct is a log encoding of AP0.
    // Packet 3.2 grading nodes compose in this seam.
    var working: vec3<f32>;
    if (p.working_kind == 0u) {{
        working = apply3(p.cg_encode, ap0);
    }} else {{
        working = vec3<f32>(
            acescct_encode(ap0.r),
            acescct_encode(ap0.g),
            acescct_encode(ap0.b),
        );
    }}
    var graded_ap0: vec3<f32>;
    if (p.working_kind == 0u) {{
        graded_ap0 = apply3(p.cg_decode, working);
    }} else {{
        graded_ap0 = vec3<f32>(
            acescct_decode(working.r),
            acescct_decode(working.g),
            acescct_decode(working.b),
        );
    }}
    let xyz_d65 = apply3(p.odt_matrix, graded_ap0);
    let display_linear = apply3(p.display_matrix, xyz_d65);
    let encoded = vec3<f32>(
        display_encode(display_linear.r),
        display_encode(display_linear.g),
        display_encode(display_linear.b),
    );
    textureStore(output_tex, gid.xy, vec4<f32>(encoded, 1.0f));
}}
"#,
        logc4_a = f64_fmt(logc4_a),
        logc4_s = f64_fmt(logc4_s),
        logc4_t = f64_fmt(logc4_t),
        logc4_b = f64_fmt(ocio_transforms::LOGC4_B),
        logc4_c = f64_fmt(ocio_transforms::LOGC4_C),
        log3g10_a = f64_fmt(LOG3G10_A),
        log3g10_b = f64_fmt(LOG3G10_B),
        log3g10_c = f64_fmt(LOG3G10_C),
        log3g10_g = f64_fmt(LOG3G10_G),
        bmd_a = f64_fmt(BMDFILM_GEN5_A),
        bmd_b = f64_fmt(BMDFILM_GEN5_B),
        bmd_c = f64_fmt(BMDFILM_GEN5_C),
        bmd_d = f64_fmt(BMDFILM_GEN5_D),
        bmd_e = f64_fmt(BMDFILM_GEN5_E),
        bmd_lin_cut = f64_fmt(BMDFILM_GEN5_LIN_CUT),
        apple_r0 = f64_fmt(APPLE_LOG_R0),
        apple_rt = f64_fmt(APPLE_LOG_RT),
        apple_sigma = f64_fmt(APPLE_LOG_SIGMA),
        apple_beta = f64_fmt(APPLE_LOG_BETA),
        apple_gamma = f64_fmt(APPLE_LOG_GAMMA),
        apple_delta = f64_fmt(APPLE_LOG_DELTA),
        acescct_cut = f64_fmt(ocio_transforms::ACESCCT_CUT),
        acescct_a = f64_fmt(ocio_transforms::ACESCCT_A),
        acescct_b = f64_fmt(ocio_transforms::ACESCCT_B),
        pq_m1 = f64_fmt(ocio_transforms::PQ_M1),
        pq_m2 = f64_fmt(ocio_transforms::PQ_M2),
        pq_c1 = f64_fmt(ocio_transforms::PQ_C1),
        pq_c2 = f64_fmt(ocio_transforms::PQ_C2),
        pq_c3 = f64_fmt(ocio_transforms::PQ_C3),
    )
}

/// Compiled IDT → working → ODT compute pipeline over `Rgba32Float`
/// textures.
pub struct OcioPipeline {
    pipeline: wgpu::ComputePipeline,
    bind_group_layout: wgpu::BindGroupLayout,
}

impl OcioPipeline {
    /// Compiles the fixed WGSL program against the context's device.
    /// The program text is compile-time; naga validation failures are build
    /// errors surfaced by the test-suite, never runtime input.
    pub fn new(ctx: &GpuContext) -> Result<Self, GpuContextError> {
        let source = wgsl_source();
        let module =
            ctx.device()
                .create_shader_module(wgpu::ShaderModuleDescriptor {
                    label: Some("saturn-render ocio pipeline"),
                    source: wgpu::ShaderSource::Wgsl(Cow::Owned(source)),
                });
        let bind_group_layout =
            ctx.device()
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("saturn-render ocio pipeline bind layout"),
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
                        wgpu::BindGroupLayoutEntry {
                            binding: 2,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: Some(NonZeroU64::new(272).unwrap()),
                            },
                            count: None,
                        },
                    ],
                });
        let pipeline_layout =
            ctx.device()
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("saturn-render ocio pipeline layout"),
                    bind_group_layouts: &[Some(&bind_group_layout)],
                    immediate_size: 0,
                });
        let pipeline =
            ctx.device()
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("saturn-render ocio pipeline"),
                    layout: Some(&pipeline_layout),
                    module: &module,
                    entry_point: Some("main"),
                    compilation_options: Default::default(),
                    cache: None,
                });
        Ok(Self {
            pipeline,
            bind_group_layout,
        })
    }

    /// Runs the transform over an `Rgba32Float` input texture and returns a
    /// new `Rgba32Float` output texture holding display-encoded values.
    pub fn transform_texture(
        &self,
        ctx: &GpuContext,
        input: &wgpu::Texture,
        config: OcioConfig,
    ) -> Result<wgpu::Texture, GpuContextError> {
        let size = input.size();
        if input.format() != wgpu::TextureFormat::Rgba32Float {
            return Err(GpuContextError::Readback(format!(
                "ocio pipeline input must be Rgba32Float, got {:?}",
                input.format()
            )));
        }
        let output = ctx.create_rgba32float_texture(
            size.width,
            size.height,
            wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::COPY_SRC,
        );
        let words = pack_params(config, size.width, size.height);
        let uniform = ctx.device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("saturn-render ocio params"),
            size: 272,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        ctx.queue()
            .write_buffer(&uniform, 0, crate::gpu_context::u32_words_as_bytes(&words));
        let input_view = input.create_view(&wgpu::TextureViewDescriptor::default());
        let output_view = output.create_view(&wgpu::TextureViewDescriptor::default());
        let bind_group = ctx.device().create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("saturn-render ocio bind group"),
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
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: uniform.as_entire_binding(),
                },
            ],
        });
        let mut encoder = ctx
            .device()
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("saturn-render ocio dispatch"),
            });
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("saturn-render ocio pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            let groups_x = size.width.div_ceil(8);
            let groups_y = size.height.div_ceil(8);
            pass.dispatch_workgroups(groups_x, groups_y, 1);
        }
        ctx.queue().submit(Some(encoder.finish()));
        Ok(output)
    }

    /// Convenience: transforms in-memory RGBA f32 pixels through the GPU and
    /// reads the result back.
    pub fn transform_rgba(
        &self,
        ctx: &GpuContext,
        width: u32,
        height: u32,
        pixels: &[f32],
        config: OcioConfig,
    ) -> Result<Vec<f32>, GpuContextError> {
        let input = ctx.create_rgba32float_texture(
            width,
            height,
            wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        );
        ctx.upload_rgba32float(&input, width, height, pixels);
        let output = self.transform_texture(ctx, &input, config)?;
        ctx.readback_rgba32float(&output, width, height)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Creates a GPU context, or reports plainly when the host has no Vulkan
    /// adapter (an explicit skip, not a fabricated pass).
    fn gpu() -> Option<GpuContext> {
        match GpuContext::new() {
            Ok(ctx) => Some(ctx),
            Err(GpuContextError::NoVulkanAdapter(reason)) => {
                eprintln!("skipping GPU test: no Vulkan adapter ({reason})");
                None
            }
            Err(e) => panic!("GPU context creation failed unexpectedly: {e}"),
        }
    }

    /// Input pixels: neutral log code values plus a colour offset patch.
    fn sample_pixels() -> Vec<f32> {
        let cvs: [f32; 4] = [0.0, 0.2784, 0.4106, 0.4883];
        let mut px = Vec::new();
        for &cv in &cvs {
            let g = (cv * 0.96 + 0.01).min(1.0);
            let b = (cv * 1.04).min(1.5);
            px.extend_from_slice(&[cv, g, b, 1.0]);
            px.extend_from_slice(&[b, cv, g, 1.0]);
        }
        px
    }

    #[test]
    fn uniform_packing_layout_is_stable() {
        let config = OcioConfig::default_editing();
        let words = pack_params(config, 192, 108);
        assert_eq!(words[0], 0, "rec709_video idt kind");
        assert_eq!(words[1], 0, "aces_cg working kind");
        assert_eq!(words[2], 0, "rec709 odt kind");
        assert_eq!(words[4], 192);
        assert_eq!(words[5], 108);
        // Matrix region starts at word 8; idt row 0 xyz matches the folded
        // camera matrix via f32 bit patterns (within f32 casting precision).
        let m = idt_matrix(ocio_transforms::CameraIdt::Rec709Video);
        for (c, w) in [8usize, 9, 10].iter().enumerate() {
            let bits = f32::from_bits(words[*w]) as f64;
            assert!(
                (bits - m[0][c]).abs() < 1e-6,
                "word {w} = {bits} vs {}",
                m[0][c]
            );
        }
        assert_eq!(words[11], 0, "row pad");
        // A non-default config must select the WGSL switch kinds by u32.
        let cfg2 = OcioConfig {
            idt: ocio_transforms::CameraIdt::ArriLogC4,
            working: WorkingSpace::AcesCct,
            odt: DisplayOdt::Rec2020Pq,
        };
        let words2 = pack_params(cfg2, 8, 1);
        assert_eq!(words2[0], 1, "arri_logc4 idt kind (u32, not f32 bits)");
        assert_eq!(words2[1], 1, "aces_cct working kind");
        assert_eq!(words2[2], 2, "rec2020_pq odt kind");
    }

    #[test]
    fn wgsl_constants_match_rust_constants() {
        let src = wgsl_source();
        for needle in [
            format!(
                "const LOGC4_A: f32 = {};",
                f64_fmt(ocio_transforms::logc4_consts().0)
            ),
            format!(
                "const ACESCCT_B: f32 = {}f;",
                f64_fmt(ocio_transforms::ACESCCT_B)
            ),
            format!("const PQ_M2: f32 = {}f;", f64_fmt(ocio_transforms::PQ_M2)),
            format!("const BMD_A: f32 = {}f;", f64_fmt(BMDFILM_GEN5_A)),
            format!(
                "const APPLE_DELTA: f32 = {}f;",
                f64_fmt(APPLE_LOG_DELTA)
            ),
        ] {
            assert!(
                src.contains(&needle),
                "WGSL missing constant line: {needle}"
            );
        }
    }

    #[test]
    fn gpu_full_pipeline_matches_cpu_reference() {
        let Some(ctx) = gpu() else { return };
        let pipeline = OcioPipeline::new(&ctx).expect("pipeline compiles");
        let width = 8u32;
        let height = 1u32;
        let pixels = sample_pixels();
        assert_eq!(pixels.len(), (width * height * 4) as usize);

        let idts = [
            ocio_transforms::CameraIdt::Rec709Video,
            ocio_transforms::CameraIdt::ArriLogC4,
            ocio_transforms::CameraIdt::RedLog3G10,
            ocio_transforms::CameraIdt::SonySLog3,
            ocio_transforms::CameraIdt::BlackmagicGen5,
            ocio_transforms::CameraIdt::AppleLog,
        ];
        for idt in idts {
            for working in [WorkingSpace::AcesCg, WorkingSpace::AcesCct] {
                for odt in [DisplayOdt::Rec709, DisplayOdt::Rec2020Pq] {
                    let config = OcioConfig { idt, working, odt };
                    let out = pipeline
                        .transform_rgba(&ctx, width, height, &pixels, config)
                        .expect("GPU transform succeeds");
                    for px in 0..(width * height) as usize {
                        let cv = [
                            pixels[px * 4] as f64,
                            pixels[px * 4 + 1] as f64,
                            pixels[px * 4 + 2] as f64,
                        ];
                        let want = transform_reference(cv, config);
                        for c in 0..3 {
                            let got = out[px * 4 + c];
                            let diff = (got - want[c] as f32).abs();
                            assert!(
                                diff <= 2e-4,
                                "{idt:?} {working:?} {odt:?} px{px} ch{c}: gpu {got} vs cpu {} (diff {diff})",
                                want[c]
                            );
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn gpu_32bit_fidelity_roundtrip() {
        let Some(ctx) = gpu() else { return };
        let width = 4u32;
        let height = 1u32;
        // Values far below half-float epsilon: only a true 32-bit path keeps
        // them (1e-9 would crush to zero in a 16-bit pipeline).
        let pixels: Vec<f32> = vec![1e-9, 2.5e-8, 0.18, 1.0, 1e-9, 2.5e-8, 0.18, 1.0, 1e-9, 2.5e-8, 0.18, 1.0, 1e-9, 2.5e-8, 0.18, 1.0];
        let texture = ctx.create_rgba32float_texture(
            width,
            height,
            wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::COPY_SRC,
        );
        ctx.upload_rgba32float(&texture, width, height, &pixels);
        let back = ctx
            .readback_rgba32float(&texture, width, height)
            .expect("readback");
        for (a, b) in back.iter().zip(pixels.iter()) {
            assert_eq!(a, b, "Rgba32Float texture must round-trip exactly");
        }
    }
}
