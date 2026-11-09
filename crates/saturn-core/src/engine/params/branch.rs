// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
use super::*;

#[derive(Deserialize)]
pub(in crate::engine) struct BranchCreateParams {
    pub(in crate::engine) name: String,
    #[serde(default)]
    pub(in crate::engine) note: String,
}

#[derive(Deserialize)]
pub(in crate::engine) struct BranchSwitchParams {
    pub(in crate::engine) name: String,
}

#[derive(Deserialize)]
pub(in crate::engine) struct BranchDiffParams {
    pub(in crate::engine) branch: String,
}

#[derive(Deserialize)]
pub(in crate::engine) struct BranchMergeParams {
    pub(in crate::engine) branch: String,
    /// v1 semantics (plan review §4.11): the whole branch is accepted
    /// (absorbed into the live state, record removed) or rejected
    /// (record discarded). No range merging until a written conflict rule.
    pub(in crate::engine) action: BranchMergeAction,
    /// When true, overrides stale baseline checks and discards divergent work.
    #[serde(default)]
    pub(in crate::engine) force: bool,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(in crate::engine) enum BranchMergeAction {
    Accept,
    Reject,
}

#[derive(Deserialize)]
pub(in crate::engine) struct FingerprintMediaParams {
    /// Restrict the scan to these asset ids; empty means every asset.
    #[serde(default)]
    pub(in crate::engine) media_ids: Vec<u64>,
}
