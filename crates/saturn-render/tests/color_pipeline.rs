// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Integration scaffold for the Packet 3.1 colour pipeline (engine-facing
//! surface, not unit internals): every shipped IDT decodes an 18% grey card
//! display-neutral through the full `IDT -> working -> ODT` chain.
//!
//! Packet 3.2 extends this file with exposure/OOZ coverage once the
//! tone-building blocks land (noted in
//! `docs/AI_NATIVE_PLAN_CHECKLIST.md` Phase 3). Serde-name stability of the
//! selections is pinned in `saturn_core::engine::tests::ocio` (single source).

use saturn_core::color::transforms::{
    apple_log_encode, bmd_gen5_encode, log3g10_encode, logc4_encode, rec709_oetf_encode,
    slog3_encode,
};
use saturn_core::color::{CameraIdt, DisplayOdt, WorkingSpace};
use saturn_render::ocio_pipeline::{transform_reference, OcioConfig};

const ALL_IDTS: &[CameraIdt] = CameraIdt::ALL;

/// 18% scene-linear grey encoded through each camera's own transfer — the
/// same construction the core curve tests use, so no anchor digits are
/// duplicated here.
fn camera_grey_cv(idt: CameraIdt) -> [f64; 3] {
    let cv: f64 = match idt {
        CameraIdt::Rec709Video => rec709_oetf_encode(0.18),
        CameraIdt::ArriLogC4 => logc4_encode(0.18),
        CameraIdt::RedLog3G10 => log3g10_encode(0.18),
        CameraIdt::SonySLog3 => slog3_encode(0.18),
        CameraIdt::BlackmagicGen5 => bmd_gen5_encode(0.18),
        // Apple Log is defined on reflection; the IDT divides by 0.9 on
        // decode, mirroring the documented `E = R * 0.9` relationship.
        CameraIdt::AppleLog => apple_log_encode(0.18 / 0.9),
    };
    [cv, cv, cv]
}

#[test]
fn mid_grey_is_display_neutral_for_every_selection_combination() {
    for idt in ALL_IDTS {
        for working in [WorkingSpace::AcesCg, WorkingSpace::AcesCct] {
            for odt in [DisplayOdt::Rec709, DisplayOdt::DisplayP3, DisplayOdt::Rec2020Pq] {
                let config = OcioConfig {
                    idt: *idt,
                    working,
                    odt,
                };
                let out = transform_reference(camera_grey_cv(*idt), config);
                // Chroma of the neutral card must vanish; luminance rides the
                // display transfer (not asserted here — Packet 3.2 exposure).
                let chroma = (out[0] - out[1]).abs().max((out[1] - out[2]).abs());
                assert!(
                    chroma < 2e-4,
                    "{idt:?} {working:?} {odt:?}: grey card shows chroma {chroma} ({out:?})"
                );
            }
        }
    }
}
