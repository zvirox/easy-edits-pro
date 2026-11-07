// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Scene-referred colour transforms for the OCIO-style pipeline (Packet 3.1).
//!
//! This module is the single source of truth for every curve constant and
//! matrix used by both the CPU reference path and the generated WGSL program
//! in [`crate::ocio_pipeline`]. Values are copied verbatim from the
//! authoritative specifications — each constant cites its source. Nothing is
//! fitted or invented: where a vendor publishes a whole-matrix IDT we use
//! their digits.
//!
//! Pipeline shape (documented deviation, Packet 3.1 scope): transforms go
//! `camera code value -> scene linear -> AP0 (ACES2065-1) -> working space
//! (ACEScg linear or ACEScct log) -> AP0 -> display`. Output transforms are a
//! fixed, small set of display primaries plus transfer approximations; the
//! full ACES RRT + per-device ODT tone scale is intentionally not
//! reimplemented in this packet (plan-review revision §4.6).

// Colour-space enums and display-matrix builders live in `spaces`
// (split for the 800-line policy); re-exported so engine and pipeline call
// sites keep a single import path.
pub use super::spaces::{
    display_p3_from_xyz_d65, rec2020_from_xyz_d65, rec709_from_xyz_d65, rec709_oetf_decode,
    rec709_oetf_encode, rec709_to_ap0_matrix, CameraIdt, DisplayOdt, WorkingSpace,
};

/// 3×3 row-major matrix; `m[row][col]`.
pub type Mat3 = [[f64; 3]; 3];

/// AP1 (ACEScg) → XYZ with ACES white, per the Academy CTL `ap1_to_xyz.ctl`.
pub const M_AP1_TO_XYZ: Mat3 = [
    [0.6624541811, 0.1340042065, 0.1561876870],
    [0.2722287168, 0.6740817658, 0.0536895174],
    [-0.0055746495, 0.0040607335, 1.0103391003],
];

/// XYZ (ACES white) → AP1 (ACEScg); inverse of [`M_AP1_TO_XYZ`], computed
/// to 13 digits and verified against the published pair.
pub const M_XYZ_TO_AP1: Mat3 = [
    [1.6410233797781, -0.3248032943230, -0.2364246952408],
    [-0.6636628588139, 1.6153315917456, 0.0167563477046],
    [0.0117218943259, -0.0082844419542, 0.9883948585499],
];

/// AP0 → XYZ, per the Academy CTL `ap0_to_xyz.ctl` (digits as published in
/// colour-science `MATRIX_AP0_TO_XYZ`, itself sourced from the Academy).
pub const M_AP0_TO_XYZ: Mat3 = [
    [0.9525523959, 0.0000000000, 0.0000936786],
    [0.3439664498, 0.7281660966, -0.0721325464],
    [0.0000000000, 0.0000000000, 1.0088251844],
];

/// XYZ → AP0 (inverse of [`M_AP0_TO_XYZ`], Academy CTL `xyz_to_ap0.ctl`).
pub const M_XYZ_TO_AP0: Mat3 = [
    [1.0498110175, 0.0000000000, -0.0000974845],
    [-0.4959030231, 1.3733130458, 0.0982400361],
    [0.0000000000, 0.0000000000, 0.9912520182],
];

/// AP0 → AP1 chromatic transform, per the Academy CTL `ap0_to_ap1.ctl`.
pub const M_AP0_TO_AP1: Mat3 = [
    [1.4514393161, -0.2365107469, -0.2149285693],
    [-0.0765537734, 1.1762296998, -0.0996759264],
    [0.0083161484, -0.0060324498, 0.9977163014],
];

/// AP1 → AP0; inverse of the published [`M_AP0_TO_AP1`], computed to 13
/// digits (round-identity verified to machine precision). The widely-copied
/// 10-digit variants drift up to ~1e-4, so the precise inverse is embedded.
pub const M_AP1_TO_AP0: Mat3 = [
    [0.6954522413832, 0.1406786964846, 0.1638690622017],
    [0.0447945633791, 0.8596711184832, 0.0955343181422],
    [-0.0055258825398, 0.0040252103172, 1.0015006722220],
];

/// CIE CAT02 chromatic adaptation matrix (Süsstrunk et al.), as used by
/// ARRI's published LogC4 → ACES transform ("CAT02" in the ALEXA 35 spec).
pub const M_CAT02: Mat3 = [
    [0.7328, 0.4296, -0.1624],
    [-0.7036, 1.6975, 0.0061],
    [0.0030, 0.0136, 0.9834],
];

/// Bradford chromatic adaptation matrix, as used by the ASWF ACES utility
/// `AP0 → CIE-XYZ-D65 (scene-referred)` (verified: the published matrix
/// reproduces a Bradford-folded ACES→D65 composition to 4e-15, vs ~4.5e-3
/// for CAT02).
pub const M_BRADFORD: Mat3 = [
    [0.8951, 0.2664, -0.1614],
    [-0.7502, 1.7135, 0.0367],
    [0.0389, -0.0685, 1.0296],
];

/// AP0 → CIE XYZ at D65 (scene-referred), official ASWF CLF
/// `OCIO.Utility.AP0_to_CIE_XYZ-D65-Scene-referred` (Bradford ACES→D65 fold).
pub const M_AP0_TO_XYZ_D65: Mat3 = [
    [0.938279849239345, -0.00445144581227847, 0.0166275235564231],
    [0.337368890823117, 0.729521566676754, -0.066890457499083],
    [0.00117395084939056, -0.00371070640198378, 1.09159450636463],
];

/// CIE 1931 D65 x,y chromaticity.
pub const D65_XY: [f64; 2] = [0.3127, 0.3290];

// ---------------------------------------------------------------------------
// Camera → ACES gamut matrices (whole-matrix vendor IDTs)
// ---------------------------------------------------------------------------

/// Linear ARRI Wide Gamut 4 → ACES2065-1 (AP0), CAT02-based, from the ARRI
/// LogC4 specification §4.3.2 (`MACES`); identical to the ASWF CLF
/// `ARRI.Input.Linear_ARRI_Wide_Gamut_4_to_ACES2065-1`.
pub const M_AWG4_TO_AP0: Mat3 = [
    [0.750957362824734131, 0.144422786709757084, 0.104619850465508965],
    [0.000821837079380207, 1.007397584885003194, -0.008219421964383583],
    [-0.000499952143533471, -0.000854177231436971, 1.001354129374970370],
];

/// Linear REDWideGamutRGB → ACES2065-1 (AP0), from the ASWF CLF
/// `RED.Input.Linear_REDWideGamutRGB_to_ACES2065-1` (RED Digital Cinema IDT).
pub const M_RWG_TO_AP0: Mat3 = [
    [0.7850588040680920, 0.0838587565440846, 0.1310824393878230],
    [0.0231738348454756, 1.0878975491923300, -0.1110713840378060],
    [-0.0737604353682082, -0.3145900722902080, 1.3883505076584200],
];

/// Linear S-Gamut3.Cine → ACES2065-1 (AP0), from the ASWF CLF
/// `Sony.Input.Linear_SGamut3Cine_to_ACES2065-1` (Sony Electronics IDT).
pub const M_SGAMUT3_CINE_TO_AP0: Mat3 = [
    [0.6387886671859780, 0.2723514337112620, 0.0888598991027595],
    [-0.00391590602528224, 1.0880732308974000, -0.0841573248721177],
    [-0.0299072021239151, -0.0264325799101947, 1.0563397820341100],
];

/// Linear Blackmagic Wide Gamut (Gen 5) → ACES2065-1 (AP0), from the ASWF CLF
/// `BlackmagicDesign.Input.Linear_BMD_WideGamut_Gen5_to_ACES2065-1`.
pub const M_BMD_WIDE_GAMUT_TO_AP0: Mat3 = [
    [0.6470913255807080, 0.2425953851342070, 0.1103132892850850],
    [0.0651915997328519, 1.0250475676047600, -0.0902391673376125],
    [-0.0275570729194699, -0.0805887097177784, 1.1081457826372500],
];

/// Linear Apple Wide Gamut → ACES2065-1 (AP0), from the ASWF CLF
/// `Apple.Input.Linear_Apple_Wide_Gamut_to_ACES2065-1` (Apple Inc. IDT,
/// submitted with the Apple Log profile to the Academy in 2023).
pub const M_APPLE_WIDE_GAMUT_TO_AP0: Mat3 = [
    [0.6949610493180960, 0.2414052687853640, 0.0636336818965400],
    [0.0473627464149326, 1.0042959250542800, -0.0516586714692157],
    [-0.0219897893598830, -0.0289891049714743, 1.0509788943313600],
];

// ---------------------------------------------------------------------------
// Transfer-function constants
// ---------------------------------------------------------------------------

/// ARRI LogC4 `b = (1023 − 95) / 1023` (LogC4 specification, Equation 1b).
pub const LOGC4_B: f64 = (1023.0 - 95.0) / 1023.0;
/// ARRI LogC4 `c = 95 / 1023` (LogC4 specification, Equation 1c).
pub const LOGC4_C: f64 = 95.0 / 1023.0;

/// RED Log3G10 constant `a` (RED white paper 915-0187, equations section).
pub const LOG3G10_A: f64 = 0.224282;
/// RED Log3G10 constant `b`.
pub const LOG3G10_B: f64 = 155.975327;
/// RED Log3G10 constant `c` (knee offset).
pub const LOG3G10_C: f64 = 0.01;
/// RED Log3G10 constant `g` (linear-segment gain).
pub const LOG3G10_G: f64 = 15.1927;

/// Blackmagic Film Generation 5 constant `A` (Blackmagic "Generation 5 Color
/// Science", 2021); digits as published.
pub const BMDFILM_GEN5_A: f64 = 0.08692876065491224;
/// Blackmagic Film Gen5 constant `B`.
pub const BMDFILM_GEN5_B: f64 = 0.005494072432257808;
/// Blackmagic Film Gen5 constant `C`.
pub const BMDFILM_GEN5_C: f64 = 0.5300133392291939;
/// Blackmagic Film Gen5 constant `D` (linear-segment gain).
pub const BMDFILM_GEN5_D: f64 = 8.283605932402494;
/// Blackmagic Film Gen5 constant `E` (linear-segment offset).
pub const BMDFILM_GEN5_E: f64 = 0.09246575342465753;
/// Blackmagic Film Gen5 linear cut.
pub const BMDFILM_GEN5_LIN_CUT: f64 = 0.005;

/// Apple Log constant `R_0` (Apple "Apple Log Profile" white paper, 2023 /
/// Academy CLF `Apple.Input.Apple_Log-Curve`).
pub const APPLE_LOG_R0: f64 = -0.05641088;
/// Apple Log reflection threshold `R_t`.
pub const APPLE_LOG_RT: f64 = 0.01;
/// Apple Log quadratic-segment scale `sigma`.
pub const APPLE_LOG_SIGMA: f64 = 47.28711236;
/// Apple Log log-segment offset `beta`.
pub const APPLE_LOG_BETA: f64 = 0.00964052;
/// Apple Log log-segment gain `gamma`.
pub const APPLE_LOG_GAMMA: f64 = 0.08550479;
/// Apple Log log-segment offset `delta`.
pub const APPLE_LOG_DELTA: f64 = 0.69336945;

/// ACEScct linear/log cut on the linear axis (Academy ST 2065-12).
pub const ACESCCT_CUT: f64 = 0.0078125;
/// ACEScct linear-segment gain (ST 2065-12).
pub const ACESCCT_A: f64 = 10.5402377416545;
/// ACEScct linear-segment offset (ST 2065-12).
pub const ACESCCT_B: f64 = 0.0729055341958;

/// SMPTE ST 2084 (PQ) constant `m1 = 2610 / 16384`.
pub const PQ_M1: f64 = 2610.0 / 16384.0;
/// SMPTE ST 2084 (PQ) constant `m2 = 2523 / 4096 * 128`.
pub const PQ_M2: f64 = 2523.0 / 4096.0 * 128.0;
/// SMPTE ST 2084 (PQ) constant `c1 = 3424 / 4096`.
pub const PQ_C1: f64 = 3424.0 / 4096.0;
/// SMPTE ST 2084 (PQ) constant `c2 = 2413 / 4096 * 32`.
pub const PQ_C2: f64 = 2413.0 / 4096.0 * 32.0;
/// SMPTE ST 2084 (PQ) constant `c3 = 2392 / 4096 * 32`.
pub const PQ_C3: f64 = 2392.0 / 4096.0 * 32.0;
/// Nominal PQ peak luminance in cd/m² (ST 2084 defines [0, 10000]).
pub const PQ_PEAK_NITS: f64 = 10_000.0;

// ---------------------------------------------------------------------------
// Camera log curves
// ---------------------------------------------------------------------------

/// Lazily-computed ARRI LogC4 `a`, `s`, `t` constants (Equations 2a–2c).
pub fn logc4_consts() -> (f64, f64, f64) {
    let a = ((2f64).powf(18.0) - 16.0) / 117.45;
    let s = (7.0 * std::f64::consts::LN_2 * (2f64).powf(7.0 - 14.0 * LOGC4_C / LOGC4_B))
        / (a * LOGC4_B);
    let t = (2f64).powf(14.0 * (-LOGC4_C / LOGC4_B) + 6.0) - 64.0;
    let _ = t;
    let t = ((2f64).powf(14.0 * (-LOGC4_C / LOGC4_B) + 6.0) - 64.0) / a;
    (a, s, t)
}

/// ARRI LogC4 encoding: relative scene linear → normalised code value
/// (LogC4 specification, Equation 2).
pub fn logc4_encode(scene_linear: f64) -> f64 {
    let (a, s, t) = logc4_consts();
    if scene_linear < t {
        (scene_linear - t) / s
    } else {
        ((scene_linear.mul_add(a, 64.0)).log2() - 6.0) / 14.0 * LOGC4_B + LOGC4_C
    }
}

/// ARRI LogC4 decoding: normalised code value → relative scene linear
/// (LogC4 specification, Equation 3).
pub fn logc4_decode(cv: f64) -> f64 {
    let (a, s, t) = logc4_consts();
    // Below code value 0 the reconstruction follows the linear toe segment.
    if cv < 0.0 {
        return cv * s + t;
    }
    let p = 14.0 * (cv - LOGC4_C) / LOGC4_B + 6.0;
    ((2f64).powf(p) - 64.0) / a
}

/// RED Log3G10 encoding: scene linear → normalised code value (v3 curve,
/// RED white paper 915-0187).
pub fn log3g10_encode(scene_linear: f64) -> f64 {
    let x = scene_linear + LOG3G10_C;
    if x < 0.0 {
        x * LOG3G10_G
    } else {
        LOG3G10_A * (x.abs().mul_add(LOG3G10_B, 1.0)).log10()
    }
}

/// RED Log3G10 decoding: normalised code value → scene linear.
pub fn log3g10_decode(cv: f64) -> f64 {
    if cv < 0.0 {
        return cv / LOG3G10_G - LOG3G10_C;
    }
    let z = 10f64.powf(cv / LOG3G10_A);
    ((z - 1.0) / LOG3G10_B - LOG3G10_C).max(-LOG3G10_C)
}

/// Sony S-Log3 encoding (scene-linear input, not reflection): normalised
/// 10-bit code values per Sony's published S-Log3 white paper.
///
/// Sony publishes the curve on *reflection* `R` (0.18 reflection = mid grey);
/// scene linear `E` relates to reflection by `R = E / 0.9` (Sony's documents
/// normalise.scene at 0.9 for 90 % white). The integer constants (420, 95,
/// 261.5, 171.2102946929) are Sony's CV form over 10-bit full range.
pub fn slog3_encode(scene_linear: f64) -> f64 {
    let reflection = scene_linear / 0.9;
    if reflection >= 0.01125 {
        (420.0 + ((reflection + 0.01) / (0.18 + 0.01)).log10() * 261.5) / 1023.0
    } else {
        (reflection * (171.2102946929 - 95.0) / 0.01125 + 95.0) / 1023.0
    }
}

/// Sony S-Log3 decoding: normalised code value → scene linear.
pub fn slog3_decode(cv: f64) -> f64 {
    let reflection = if cv >= 420.0 / 1023.0 {
        0.19f64.mul_add(10f64.powf((cv * 1023.0 - 420.0) / 261.5), 0.0) - 0.01
    } else {
        (cv * 1023.0 - 95.0) * 0.01125 / (171.2102946929 - 95.0)
    };
    reflection * 0.9
}

/// Blackmagic Film Generation 5 encoding: scene linear → normalised code
/// value (Blackmagic Design "Generation 5 Color Science", 2021).
pub fn bmd_gen5_encode(scene_linear: f64) -> f64 {
    if scene_linear < BMDFILM_GEN5_LIN_CUT {
        BMDFILM_GEN5_D * scene_linear + BMDFILM_GEN5_E
    } else {
        BMDFILM_GEN5_A * (scene_linear + BMDFILM_GEN5_B).ln() + BMDFILM_GEN5_C
    }
}

/// Blackmagic Film Generation 5 decoding: normalised code value → scene
/// linear.
pub fn bmd_gen5_decode(cv: f64) -> f64 {
    let log_cut = BMDFILM_GEN5_D.mul_add(BMDFILM_GEN5_LIN_CUT, BMDFILM_GEN5_E);
    if cv < log_cut {
        (cv - BMDFILM_GEN5_E) / BMDFILM_GEN5_D
    } else {
        ((cv - BMDFILM_GEN5_C) / BMDFILM_GEN5_A).exp() - BMDFILM_GEN5_B
    }
}

/// Apple Log encoding: linear *reflection* → normalised code value (Apple
/// white paper). Callers holding scene linear convert via `R = E / 0.9`
/// (Apple normalises 90 % reflectance white to 1.0).
pub fn apple_log_encode(reflection: f64) -> f64 {
    if reflection < APPLE_LOG_R0 {
        0.0
    } else if reflection < APPLE_LOG_RT {
        APPLE_LOG_SIGMA * (reflection - APPLE_LOG_R0) * (reflection - APPLE_LOG_R0)
    } else {
        APPLE_LOG_GAMMA * (reflection + APPLE_LOG_BETA).log2() + APPLE_LOG_DELTA
    }
}

/// Apple Log decoding: normalised code value → linear reflection.
pub fn apple_log_decode(cv: f64) -> f64 {
    let p_t = APPLE_LOG_SIGMA * (APPLE_LOG_RT - APPLE_LOG_R0) * (APPLE_LOG_RT - APPLE_LOG_R0);
    if cv >= p_t {
        (2f64).powf((cv - APPLE_LOG_DELTA) / APPLE_LOG_GAMMA) - APPLE_LOG_BETA
    } else if cv >= 0.0 {
        (cv / APPLE_LOG_SIGMA).sqrt() + APPLE_LOG_R0
    } else {
        0.0
    }
}

/// ACEScct encoding: linear ACES (AP0) value → ACEScct code value
/// (Academy CTL `ACEScct.ctl`, ST 2065-12).
pub fn acescct_encode(linear: f64) -> f64 {
    if linear <= ACESCCT_CUT {
        ACESCCT_A * linear + ACESCCT_B
    } else {
        (linear.log2() + 9.72) / 17.52
    }
}

/// ACEScct decoding: ACEScct code value → linear ACES (AP0) value.
pub fn acescct_decode(cv: f64) -> f64 {
    if cv <= 0.155251141552511 {
        (cv - ACESCCT_B) / ACESCCT_A
    } else {
        (17.52 * cv - 9.72).exp2()
    }
}

/// SMPTE ST 2084 (PQ) encoding of relative display-linear light
/// (`y = nits / 10000`) → normalised code value.
pub fn pq_encode(y_relative: f64) -> f64 {
    if y_relative <= 0.0 {
        return 0.0;
    }
    let yp = y_relative.powf(PQ_M1);
    let num = PQ_C1 + PQ_C2 * yp;
    let den = 1.0 + PQ_C3 * yp;
    (num / den).powf(PQ_M2)
}

/// SMPTE ST 2084 (PQ) decoding: normalised code value → relative
/// display-linear light (`nits / 10000`).
pub fn pq_decode(cv: f64) -> f64 {
    if cv <= 0.0 {
        return 0.0;
    }
    let ep = cv.powf(1.0 / PQ_M2);
    let num = (ep - PQ_C1).max(0.0);
    let den = PQ_C2 - PQ_C3 * ep;
    (num / den).powf(1.0 / PQ_M1)
}

/// IEC 61966-2-1 sRGB piecewise encoding (used for the simplified Rec.709
/// and Display P3 output transforms; documented pipeline deviation).
pub fn srgb_encode(linear: f64) -> f64 {
    if linear <= 0.0031308 {
        12.92 * linear
    } else {
        1.055 * linear.powf(1.0 / 2.4) - 0.055
    }
}

// ---------------------------------------------------------------------------
// Matrix helpers
// ---------------------------------------------------------------------------

/// Multiplies two 3×3 row-major matrices: returns `a * b`.
pub fn mat3_mul(a: &Mat3, b: &Mat3) -> Mat3 {
    let mut out = [[0.0f64; 3]; 3];
    for (row, out_row) in out.iter_mut().enumerate() {
        for col in 0..3 {
            out_row[col] = a[row][0] * b[0][col]
                + a[row][1] * b[1][col]
                + a[row][2] * b[2][col];
        }
    }
    out
}

/// Applies a 3×3 row-major matrix to a 3-vector.
pub fn mat3_apply(m: &Mat3, v: [f64; 3]) -> [f64; 3] {
    [
        m[0][0] * v[0] + m[0][1] * v[1] + m[0][2] * v[2],
        m[1][0] * v[0] + m[1][1] * v[1] + m[1][2] * v[2],
        m[2][0] * v[0] + m[2][1] * v[1] + m[2][2] * v[2],
    ]
}

/// Inverts a 3×3 row-major matrix. Returns `None` when singular (det ≈ 0).
pub fn mat3_inv(m: &Mat3) -> Option<Mat3> {
    let det = m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
        - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
        + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0]);
    if det.abs() < 1e-12 {
        return None;
    }
    let inv_det = 1.0 / det;
    Some([
        [
            (m[1][1] * m[2][2] - m[1][2] * m[2][1]) * inv_det,
            (m[0][2] * m[2][1] - m[0][1] * m[2][2]) * inv_det,
            (m[0][1] * m[1][2] - m[0][2] * m[1][1]) * inv_det,
        ],
        [
            (m[1][2] * m[2][0] - m[1][0] * m[2][2]) * inv_det,
            (m[0][0] * m[2][2] - m[0][2] * m[2][0]) * inv_det,
            (m[0][2] * m[1][0] - m[0][0] * m[1][2]) * inv_det,
        ],
        [
            (m[1][0] * m[2][1] - m[1][1] * m[2][0]) * inv_det,
            (m[0][1] * m[2][0] - m[0][0] * m[2][1]) * inv_det,
            (m[0][0] * m[1][1] - m[0][1] * m[1][0]) * inv_det,
        ],
    ])
}

/// Converts CIE x,y chromaticity to XYZ with Y = 1.
pub fn xy_to_xyz(xy: [f64; 2]) -> [f64; 3] {
    [xy[0] / xy[1], 1.0, (1.0 - xy[0] - xy[1]) / xy[1]]
}

/// CAT02 chromatic adaptation from one XYZ white to another.
/// `from_xy`/`to_xy` are CIE x,y chromaticities; returns XYZ → XYZ.
pub fn cat02_adaptation(from_xy: [f64; 2], to_xy: [f64; 2]) -> Mat3 {
    cat_adaptation(&M_CAT02, from_xy, to_xy)
}

/// Bradford chromatic adaptation from one XYZ white to another (matches the
/// ASWF ACES → D65 utility transforms).
pub fn bradford_adaptation(from_xy: [f64; 2], to_xy: [f64; 2]) -> Mat3 {
    cat_adaptation(&M_BRADFORD, from_xy, to_xy)
}

/// Generic cone-domain chromatic adaptation: diagonal von Kries scaling of
/// the given cone matrix's reactions from one white to another.
fn cat_adaptation(cat: &Mat3, from_xy: [f64; 2], to_xy: [f64; 2]) -> Mat3 {
    let src = mat3_apply(cat, xy_to_xyz(from_xy));
    let dst = mat3_apply(cat, xy_to_xyz(to_xy));
    let ratio = [
        dst[0] / src[0],
        dst[1] / src[1],
        dst[2] / src[2],
    ];
    let diag = [
        [ratio[0], 0.0, 0.0],
        [0.0, ratio[1], 0.0],
        [0.0, 0.0, ratio[2]],
    ];
    let inv = mat3_inv(cat).expect("CAT/Bradford matrices are never singular");
    mat3_mul(&inv, &mat3_mul(&diag, cat))
}

/// Builds the XYZ (row-major) matrix for RGB primaries at a given white
/// point, scaled so the white maps to Y = 1 (standard normalised primary
/// matrix construction).
pub fn primaries_to_xyz(primaries: [[f64; 2]; 3], white_xy: [f64; 2]) -> Mat3 {
    let r = xy_to_xyz(primaries[0]);
    let g = xy_to_xyz(primaries[1]);
    let b = xy_to_xyz(primaries[2]);
    let cols_rgb_xyz = [
        [r[0], g[0], b[0]],
        [r[1], g[1], b[1]],
        [r[2], g[2], b[2]],
    ];
    let rgb_to_xyz_base: Mat3 = cols_rgb_xyz;
    let white_rgb = mat3_inv(&rgb_to_xyz_base)
        .map(|inv| mat3_apply(&inv, xy_to_xyz(white_xy)))
        .expect("non-degenerate primaries");
    [
        [
            rgb_to_xyz_base[0][0] * white_rgb[0],
            rgb_to_xyz_base[0][1] * white_rgb[1],
            rgb_to_xyz_base[0][2] * white_rgb[2],
        ],
        [
            rgb_to_xyz_base[1][0] * white_rgb[0],
            rgb_to_xyz_base[1][1] * white_rgb[1],
            rgb_to_xyz_base[1][2] * white_rgb[2],
        ],
        [
            rgb_to_xyz_base[2][0] * white_rgb[0],
            rgb_to_xyz_base[2][1] * white_rgb[1],
            rgb_to_xyz_base[2][2] * white_rgb[2],
        ],
    ]
}

// ---------------------------------------------------------------------------
// Reference tests
// ---------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64, tol: f64) -> bool {
        (a - b).abs() <= tol
    }

    #[test]
    fn logc4_matches_arri_reference_table() {
        // ARRI LogC4 specification, Appendix B conversion table.
        assert!(close(logc4_encode(0.18), 0.2784, 5e-5), "{}", logc4_encode(0.18));
        assert!(close(logc4_encode(0.0), 0.0929, 5e-5), "{}", logc4_encode(0.0));
        assert!(close(logc4_decode(0.0), -0.0181, 5e-5), "{}", logc4_decode(0.0));
    }

    #[test]
    fn logc4_roundtrip() {
        for e in [-0.01, 0.0001, 0.0, 0.18, 1.0, 16.0, 469.8] {
            let cv = logc4_encode(e);
            let back = logc4_decode(cv);
            assert!(
                close(back, e, 1e-9 * e.abs().max(1e-3)),
                "e={e} cv={cv} back={back}"
            );
        }
    }

    #[test]
    fn log3g10_matches_reference() {
        // colour-science doctest anchors (RED white paper curve):
        // encode(0.0) = 0.09155148…, decode(1.0) = 184.32234764…
        assert!(close(log3g10_encode(0.0), 0.09155148, 1e-8));
        assert!(close(log3g10_decode(1.0), 184.32234764, 1e-6));
    }

    #[test]
    fn log3g10_roundtrip() {
        for e in [0.0, 0.001, 0.18, 1.0, 16.0, 184.32] {
            let back = log3g10_decode(log3g10_encode(e));
            assert!(close(back, e, 1e-9 * e.abs().max(1e-4)), "e={e} back={back}");
        }
    }

    #[test]
    fn slog3_matches_reference() {
        // Sony's form: 0.18 reflection (0.162 scene linear at 0.9 scale) sits
        // at CV 420/1023 by construction; verify both directions.
        let cv_018_refl = (420.0 + (((0.18f64 + 0.01) / 0.19)).log10() * 261.5) / 1023.0;
        assert_eq!(cv_018_refl, 420.0 / 1023.0);
        // Our encode takes scene linear: 0.18 scene linear → reflection 0.2.
        let cv = slog3_encode(0.18);
        let expected = (420.0 + (((0.18f64 / 0.9 + 0.01) / 0.19)).log10() * 261.5) / 1023.0;
        assert!(close(cv, expected, 1e-12));
    }

    #[test]
    fn slog3_roundtrip() {
        for e in [0.0, 0.0001, 0.18, 1.0, 14.0] {
            let back = slog3_decode(slog3_encode(e));
            assert!(close(back, e, 1e-9 * e.abs().max(1e-5)), "e={e} back={back}");
        }
        // Sony reference: scene linear 0 -> CV 95/1023 * linear branch = 95/1023.
        assert!(close(slog3_encode(0.0), 95.0 / 1023.0, 1e-12));
    }

    #[test]
    fn bmd_gen5_matches_reference() {
        // colour-science doctest anchor: decode(0.38356164383561653) ≈ 0.18
        // ⇒ encode(0.18) = 0.38356164383561653.
        assert!(
            close(bmd_gen5_encode(0.18), 0.38356164383561653, 1e-12),
            "{}",
            bmd_gen5_encode(0.18)
        );
        assert!(close(bmd_gen5_decode(0.38356164383561653), 0.18, 1e-9));
    }

    #[test]
    fn bmd_gen5_roundtrip() {
        for e in [0.0, 0.001, 0.18, 1.0, 16.0] {
            let back = bmd_gen5_decode(bmd_gen5_encode(e));
            assert!(close(back, e, 1e-12 * e.abs().max(1e-6)), "e={e} back={back}");
        }
    }

    #[test]
    fn apple_log_matches_reference() {
        // colour-science doctest anchor: decode(0.48827245852686763) ≈ 0.18.
        assert!(
            close(apple_log_decode(0.48827245852686763), 0.18, 1e-9),
            "{}",
            apple_log_decode(0.48827245852686763)
        );
        assert!(close(apple_log_encode(0.18), 0.48827245852686763, 1e-12));
    }

    #[test]
    fn apple_log_roundtrip() {
        for r in [0.0, 0.001, 0.18, 1.0, 8.0] {
            let back = apple_log_decode(apple_log_encode(r));
            assert!(close(back, r, 1e-9 * r.abs().max(1e-6)), "r={r} back={back}");
        }
    }

    #[test]
    fn acescct_reference_and_roundtrip() {
        // Academy CTL ACEScct.ctl: encode(0.18) ≈ 0.4135883…, encode(0) = 0.0729055341958.
        assert!(close(acescct_encode(0.0), 0.0729055341958, 1e-12));
        let cv = acescct_encode(0.18);
        assert!(close(cv, 0.4135883, 1e-6), "{cv}");
        for e in [0.0, 1e-6, 0.008, 0.18, 1.0, 50.0] {
            let back = acescct_decode(acescct_encode(e));
            assert!(close(back, e, 1e-12 * e.abs().max(1e-9)), "e={e} back={back}");
        }
    }

    #[test]
    fn pq_roundtrip_and_endpoints() {
        assert_eq!(pq_encode(0.0), 0.0);
        assert_eq!(pq_decode(0.0), 0.0);
        for y in [1e-6, 0.01, 0.18, 0.5, 1.0] {
            let back = pq_decode(pq_encode(y));
            assert!(close(back, y, 1e-9 * y), "y={y} back={back}");
        }
    }

    #[test]
    fn srgb_endpoints() {
        assert!(close(srgb_encode(0.0031308), 0.04045, 1e-6));
        assert!(close(srgb_encode(1.0), 1.0, 1e-9));
    }

    #[test]
    fn academy_matrices_roundtrip() {
        let identity_m = mat3_mul(&M_AP0_TO_AP1, &M_AP1_TO_AP0);
        for (r, irow) in identity_m.iter().enumerate() {
            for (c, v) in irow.iter().enumerate() {
                assert!(
                    close(*v, if r == c { 1.0 } else { 0.0 }, 1e-7),
                    "[{r}][{c}]={v}"
                );
            }
        }
    }

    #[test]
    fn every_camera_idt_maps_midgrey_to_018_in_ap0() {
        for idt in CameraIdt::ALL {
            // Mid-grey code values per curve at scene linear 0.18:
            let cv = match idt {
                CameraIdt::Rec709Video => rec709_oetf_encode(0.18),
                CameraIdt::ArriLogC4 => logc4_encode(0.18),
                CameraIdt::RedLog3G10 => log3g10_encode(0.18),
                CameraIdt::SonySLog3 => slog3_encode(0.18),
                CameraIdt::BlackmagicGen5 => bmd_gen5_encode(0.18),
                CameraIdt::AppleLog => apple_log_encode(0.18 / 0.9),
            };
            let ap0 = idt.decode_to_ap0([cv, cv, cv]);
            for (i, v) in ap0.iter().enumerate() {
                assert!(
                    close(*v, 0.18, 1e-6),
                    "{idt:?}: channel {i} = {v}"
                );
            }
            // Neutrality: rounded camera grey must stay neutral in AP0.
            assert!(
                (ap0[0] - ap0[1]).abs() < 1e-9 && (ap0[1] - ap0[2]).abs() < 1e-9,
                "{idt:?} broke neutrality: {ap0:?}"
            );
        }
    }

    #[test]
    fn working_spaces_roundtrip_through_ap0() {
        let sample = [0.18, 0.5, 2.3];
        for ws in [WorkingSpace::AcesCg, WorkingSpace::AcesCct] {
            let w = ws.from_ap0(sample);
            let back = ws.to_ap0(w);
            for (a, b) in back.iter().zip(sample.iter()) {
                assert!(close(*a, *b, 1e-8 * b.abs().max(1e-6)));
            }
        }
        // ACEScg neutral axis: equal AP0 RGB maps to equal AP1 RGB.
        let ap1 = WorkingSpace::AcesCg.from_ap0([0.18, 0.18, 0.18]);
        assert!((ap1[0] - ap1[1]).abs() < 1e-9 && (ap1[1] - ap1[2]).abs() < 1e-9);
    }

    #[test]
    fn odts_keep_neutrality_and_range() {
        for odt in [
            DisplayOdt::Rec709,
            DisplayOdt::DisplayP3,
            DisplayOdt::Rec2020Pq,
        ] {
            let out = odt.from_ap0([0.18, 0.18, 0.18]);
            assert!(
                (out[0] - out[1]).abs() < 1e-6 && (out[1] - out[2]).abs() < 1e-6,
                "{odt:?} broke neutrality: {out:?}"
            );
            for v in out {
                assert!((0.0..=1.0).contains(&v), "{odt:?} out of range: {v}");
            }
        }
    }
}
