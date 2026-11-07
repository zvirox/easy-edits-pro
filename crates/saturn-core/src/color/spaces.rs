// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Colour-space selections shared with the engine (Packet 3.1).
//!
//! The serde enums (`CameraIdt`, `WorkingSpace`, `DisplayOdt`) that the
//! engine commands and project schema serialise, plus the primaries-derived
//! display matrices. All curve constants and ACES matrices stay in
//! [`super::transforms`]; this module builds on them.

use serde::{Deserialize, Serialize};

use super::transforms::{
    acescct_decode, acescct_encode, apple_log_decode, bmd_gen5_decode, log3g10_decode,
    logc4_decode, mat3_apply, mat3_inv, mat3_mul, pq_encode, primaries_to_xyz, slog3_decode,
    srgb_encode, D65_XY, M_APPLE_WIDE_GAMUT_TO_AP0, M_AP0_TO_AP1, M_AP0_TO_XYZ_D65,
    M_AWG4_TO_AP0, M_BMD_WIDE_GAMUT_TO_AP0, M_RWG_TO_AP0, M_SGAMUT3_CINE_TO_AP0, M_XYZ_TO_AP0,
    Mat3,
};

/// Camera Input Device Transform selector. Serialised snake_case to match
/// engine command parameters.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum CameraIdt {
    /// Rec.709 video-legal input (Broadcast/DSLR footage in 709 space).
    #[default]
    #[serde(rename = "rec709_video")]
    Rec709Video,
    /// ARRI ALEXA 35 LogC4 / ARRI Wide Gamut 4.
    #[serde(rename = "arri_logc4")]
    ArriLogC4,
    /// RED Log3G10 / REDWideGamutRGB.
    #[serde(rename = "red_log3g10")]
    RedLog3G10,
    /// Sony S-Log3 / S-Gamut3.Cine.
    #[serde(rename = "sony_slog3")]
    SonySLog3,
    /// Blackmagic Film Gen 5 / Blackmagic Wide Gamut.
    #[serde(rename = "blackmagic_gen5")]
    BlackmagicGen5,
    /// Apple Log / Apple Wide Gamut.
    #[serde(rename = "apple_log")]
    AppleLog,
}

impl CameraIdt {
    /// All supported IDTs, for validation error messages.
    pub const ALL: &'static [CameraIdt] = &[
        CameraIdt::Rec709Video,
        CameraIdt::ArriLogC4,
        CameraIdt::RedLog3G10,
        CameraIdt::SonySLog3,
        CameraIdt::BlackmagicGen5,
        CameraIdt::AppleLog,
    ];

    /// Serialization names, matching the serde snake_case representation.
    pub fn as_str(self) -> &'static str {
        match self {
            CameraIdt::Rec709Video => "rec709_video",
            CameraIdt::ArriLogC4 => "arri_logc4",
            CameraIdt::RedLog3G10 => "red_log3g10",
            CameraIdt::SonySLog3 => "sony_slog3",
            CameraIdt::BlackmagicGen5 => "blackmagic_gen5",
            CameraIdt::AppleLog => "apple_log",
        }
    }

    /// Decodes normalised code values into linear AP0 (ACES2065-1).
    pub fn decode_to_ap0(self, cv: [f64; 3]) -> [f64; 3] {
        let (curve, matrix): (fn(f64) -> f64, &Mat3) = match self {
            // Rec.709 video: apply the 709 OETF inverse, then the 709 → AP0
            // gamut matrix derived from published primaries (see
            // [`rec709_to_ap0_matrix`]).
            CameraIdt::Rec709Video => (rec709_oetf_decode, &rec709_to_ap0_matrix()),
            CameraIdt::ArriLogC4 => (logc4_decode, &M_AWG4_TO_AP0),
            CameraIdt::RedLog3G10 => (log3g10_decode, &M_RWG_TO_AP0),
            CameraIdt::SonySLog3 => (slog3_decode, &M_SGAMUT3_CINE_TO_AP0),
            CameraIdt::BlackmagicGen5 => (bmd_gen5_decode, &M_BMD_WIDE_GAMUT_TO_AP0),
            CameraIdt::AppleLog => (apple_log_decode_wrapper, &M_APPLE_WIDE_GAMUT_TO_AP0),
        };
        let linear = [curve(cv[0]), curve(cv[1]), curve(cv[2])];
        mat3_apply(matrix, linear)
    }
}

/// Apple Log's curve is defined on reflection; the render pipeline holds
/// scene linear, so decode() converts reflection back through `E = R * 0.9`.
fn apple_log_decode_wrapper(cv: f64) -> f64 {
    apple_log_decode(cv) * 0.9
}

/// Rec.709 OETF inverse (IEC/BT.709 piecewise): encoded → linear.
pub fn rec709_oetf_decode(v: f64) -> f64 {
    if v < 0.081 {
        v / 4.5
    } else {
        ((v + 0.099) / 1.099).powf(1.0 / 0.45)
    }
}

/// Rec.709 OETF (linear → encoded), for completeness of the video IDT.
pub fn rec709_oetf_encode(linear: f64) -> f64 {
    let linear = linear.max(0.0);
    if linear < 0.018 {
        4.5 * linear
    } else {
        1.099 * linear.powf(0.45) - 0.099
    }
}

/// Rec.709 (D65) → AP0 chromatic matrix: 709 → XYZ (published primaries),
/// Bradford-adapted D65 → ACES white, then XYZ → AP0 (Academy matrix).
/// Derived once via `std::sync::OnceLock`.
pub fn rec709_to_ap0_matrix() -> Mat3 {
    use std::sync::OnceLock;
    static M: OnceLock<Mat3> = OnceLock::new();
    *M.get_or_init(|| {
        let primaries = [[0.640, 0.330], [0.300, 0.600], [0.150, 0.060]];
        let to_xyz = primaries_to_xyz(primaries, D65_XY);
        let aces_xy = [0.32168, 0.33767];
        let adapt = super::transforms::bradford_adaptation(D65_XY, aces_xy);
        mat3_mul(&M_XYZ_TO_AP0, &mat3_mul(&adapt, &to_xyz))
    })
}

/// Scene-referred working space selector.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkingSpace {
    /// ACEScg: AP1 primaries, linear (grading in scene linear).
    #[default]
    AcesCg,
    /// ACEScct: AP0 primaries with the log ACEScct transfer.
    AcesCct,
}

impl WorkingSpace {
    /// Serialisation names, matching serde snake_case.
    pub fn as_str(self) -> &'static str {
        match self {
            WorkingSpace::AcesCg => "aces_cg",
            WorkingSpace::AcesCct => "aces_cct",
        }
    }

    /// Converts linear AP0 into this working space.
    pub fn from_ap0(self, ap0: [f64; 3]) -> [f64; 3] {
        match self {
            WorkingSpace::AcesCg => mat3_apply(&M_AP0_TO_AP1, ap0),
            WorkingSpace::AcesCct => [
                acescct_encode(ap0[0]),
                acescct_encode(ap0[1]),
                acescct_encode(ap0[2]),
            ],
        }
    }

    /// Converts working-space values back to linear AP0.
    pub fn to_ap0(self, working: [f64; 3]) -> [f64; 3] {
        match self {
            WorkingSpace::AcesCg => mat3_apply(&super::transforms::M_AP1_TO_AP0, working),
            WorkingSpace::AcesCct => [
                acescct_decode(working[0]),
                acescct_decode(working[1]),
                acescct_decode(working[2]),
            ],
        }
    }
}

/// Display output transform selector.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DisplayOdt {
    /// Rec.709 / sRGB-piecewise SDR.
    #[default]
    Rec709,
    /// Display P3 / sRGB-piecewise SDR.
    DisplayP3,
    /// Rec.2020 primaries, SMPTE ST 2084 (PQ) transfer, 10 000-nit nominal
    /// range. Simplified mapping (no RRT tone scale): SDR white (1.0) maps
    /// to 100 nits.
    Rec2020Pq,
}

impl DisplayOdt {
    /// Serialisation names, matching serde snake_case.
    pub fn as_str(self) -> &'static str {
        match self {
            DisplayOdt::Rec709 => "rec709",
            DisplayOdt::DisplayP3 => "display_p3",
            DisplayOdt::Rec2020Pq => "rec2020_pq",
        }
    }

    /// Converts linear AP0 scene values to display-encoded values.
    /// Documented simplification: the official ASWF AP0 → XYZ-D65 fold,
    /// display primaries' matrix, and the display transfer — no RRT tone
    /// compression.
    pub fn from_ap0(self, ap0: [f64; 3]) -> [f64; 3] {
        let xyz_d65 = mat3_apply(&M_AP0_TO_XYZ_D65, ap0);
        match self {
            DisplayOdt::Rec709 => {
                let m = rec709_from_xyz_d65();
                mat3_apply(&m, xyz_d65).map(srgb_encode)
            }
            DisplayOdt::DisplayP3 => {
                let m = display_p3_from_xyz_d65();
                mat3_apply(&m, xyz_d65).map(srgb_encode)
            }
            DisplayOdt::Rec2020Pq => {
                let m = rec2020_from_xyz_d65();
                // PQ encodes nits/10000; SDR white = 100 nits (documented).
                mat3_apply(&m, xyz_d65).map(|y| pq_encode(y / 100.0))
            }
        }
    }
}

/// XYZ(D65) → Rec.709 RGB, derived from published primaries.
pub fn rec709_from_xyz_d65() -> Mat3 {
    use std::sync::OnceLock;
    static M: OnceLock<Mat3> = OnceLock::new();
    *M.get_or_init(|| {
        let to_xyz = primaries_to_xyz([[0.640, 0.330], [0.300, 0.600], [0.150, 0.060]], D65_XY);
        mat3_inv(&to_xyz).expect("Rec.709 primaries are non-degenerate")
    })
}

/// XYZ(D65) → Display P3 RGB, derived from published primaries.
pub fn display_p3_from_xyz_d65() -> Mat3 {
    use std::sync::OnceLock;
    static M: OnceLock<Mat3> = OnceLock::new();
    *M.get_or_init(|| {
        let to_xyz = primaries_to_xyz([[0.680, 0.320], [0.265, 0.690], [0.150, 0.060]], D65_XY);
        mat3_inv(&to_xyz).expect("Display P3 primaries are non-degenerate")
    })
}

/// XYZ(D65) → Rec.2020 RGB, derived from published primaries.
pub fn rec2020_from_xyz_d65() -> Mat3 {
    use std::sync::OnceLock;
    static M: OnceLock<Mat3> = OnceLock::new();
    *M.get_or_init(|| {
        let to_xyz = primaries_to_xyz([[0.708, 0.292], [0.170, 0.797], [0.131, 0.046]], D65_XY);
        mat3_inv(&to_xyz).expect("Rec.2020 primaries are non-degenerate")
    })
}
