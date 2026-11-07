// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Scene-referred colour management (AI-native plan Packet 3.1).
//!
//! [`transforms`] is the single source of truth for every curve constant and
//! ACES matrix (CPU reference path and the WGSL generation in saturn-render);
//! [`spaces`] holds the serde selections the engine commands and project
//! schema serialise. The GPU execution lives in `saturn-render` (wgpu), which
//! consumes these types and constants; project state stays owned by the core.

pub mod spaces;
pub mod transforms;

pub use spaces::{CameraIdt, DisplayOdt, WorkingSpace};
