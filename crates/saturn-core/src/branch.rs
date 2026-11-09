// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Copy-on-write timeline branches.
//!
//! A branch is a serialized snapshot of the whole [`crate::project::Project`]
//! edit state taken while the engine works on another branch of the same
//! document. Nothing is shared partially: the snapshot is a value copy, so
//! editing one branch cannot mutate another.
//!
//! Merge v1 (plan review §4.11, `docs/AI_NATIVE_PLAN_CHECKLIST.md` Packet 0.2):
//! a whole branch is accepted or rejected only. There are no per-range
//! merges; a written conflict rule must exist before those can ship.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Name reserved for the live document state; no branch record may take it.
pub const MAIN_BRANCH: &str = "main";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct TimelineBranch {
    pub id: u64,
    pub name: String,
    #[serde(default)]
    pub note: String,
    /// Name of the branch this was forked from, when known.
    #[serde(default)]
    pub parent: Option<String>,
    pub created_at_unix_ms: u64,
    /// Target baseline revision at the time of fork (plan §5).
    #[serde(default)]
    pub base_revision: u64,
    /// Current head revision of this branch (plan §5).
    #[serde(default)]
    pub head_revision: u64,
    /// Copy-on-write snapshot of the whole `Project` edit state (media
    /// metadata included, media *files* excluded): accepting a branch can
    /// carry relinked paths along, never content.
    pub state: Value,
}

impl TimelineBranch {
    /// Validate a *user-supplied* branch name (used by `branch_create` and
    /// `branch_switch` targets). `main` is rejected because users cannot
    /// create the reserved live branch; the engine itself may still park the
    /// live state in a record named `main` when switching away, and project
    /// validation accepts that one reserved record.
    pub fn validate_name(name: &str) -> Result<String, String> {
        let name = name.trim();
        if name.is_empty() {
            return Err("Branch name cannot be empty".into());
        }
        if name.len() > 120 {
            return Err("Branch name cannot exceed 120 characters".into());
        }
        if name.eq_ignore_ascii_case(MAIN_BRANCH) {
            return Err(format!(
                "Branch name `{MAIN_BRANCH}` is reserved for the live timeline"
            ));
        }
        Ok(name.to_owned())
    }
}

// ---------------------------------------------------------------------------
// Diff report types
// ---------------------------------------------------------------------------

/// Disposition of one clip slot when comparing two snapshots of the same
/// track. Index-based (insertions shift later slots), which is acceptable for
/// a summary report: the merge itself is whole-branch either way.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ClipChangeKind {
    Added,
    Removed,
    Changed,
    /// Content and duration match; only `timeline_start` differs.
    Moved,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ClipChange {
    pub track_id: u64,
    pub clip_index: usize,
    pub kind: ClipChangeKind,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Default)]
pub struct SequenceDiffReport {
    pub name: String,
    pub tracks_matched: bool,
    #[serde(default)]
    pub tracks_added: u32,
    #[serde(default)]
    pub tracks_removed: u32,
    #[serde(default)]
    pub clip_changes: Vec<ClipChange>,
    #[serde(default)]
    pub markers_changed: bool,
    #[serde(default)]
    pub graphics_changed: bool,
    #[serde(default)]
    pub captions_changed: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Default)]
pub struct MediaDiffReport {
    #[serde(default)]
    pub added_ids: Vec<u64>,
    #[serde(default)]
    pub removed_ids: Vec<u64>,
    /// Same asset id, different path: a relink the branch work did.
    #[serde(default)]
    pub relinked: Vec<RelinkChange>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct RelinkChange {
    pub id: u64,
    pub from: String,
    pub to: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct BranchDiffReport {
    pub branch: String,
    pub identical: bool,
    #[serde(default)]
    pub media: MediaDiffReport,
    #[serde(default)]
    pub sequences: Vec<SequenceDiffReport>,
}

/// Compare two serialized `Project` values. Malformed snapshots (missing
/// expected keys) simply report as changes; they never panic.
pub fn diff_projects(branch_name: &str, base: &Value, candidate: &Value) -> BranchDiffReport {
    let mut report = BranchDiffReport {
        branch: branch_name.to_owned(),
        identical: base == candidate,
        media: diff_media(base.get("media"), candidate.get("media")),
        sequences: diff_sequence_groups(base, candidate),
    };
    if report.identical {
        // An identical snapshot should also not report noise.
        report.media = MediaDiffReport::default();
        report.sequences.clear();
    }
    report
}

fn diff_media(base: Option<&Value>, candidate: Option<&Value>) -> MediaDiffReport {
    let base = as_array(base);
    let candidate = as_array(candidate);
    let get_id = |item: &Value| item.get("id").and_then(Value::as_u64);
    let by_id: std::collections::HashMap<u64, &Value> =
        base.iter().filter_map(|item| get_id(item).map(|id| (id, item))).collect();
    let candidate_ids: std::collections::HashSet<u64> =
        candidate.iter().filter_map(|item| get_id(item)).collect();

    let mut report = MediaDiffReport::default();
    for item in candidate {
        let Some(id) = get_id(item) else { continue };
        match by_id.get(&id) {
            None => report.added_ids.push(id),
            Some(previous) => {
                let from = as_string(previous.get("path"));
                let to = as_string(item.get("path"));
                if from != to {
                    report.relinked.push(RelinkChange { id, from, to });
                }
            }
        }
    }
    for item in base {
        if let Some(id) = get_id(item) {
            if !candidate_ids.contains(&id) {
                report.removed_ids.push(id);
            }
        }
    }
    report
}

fn diff_sequence_groups(base: &Value, candidate: &Value) -> Vec<SequenceDiffReport> {
    let mut reports = Vec::new();
    let base_main = base.get("sequence");
    let candidate_main = candidate.get("sequence");
    if base_main.is_some() || candidate_main.is_some() {
        reports.push(diff_sequence(
            as_string(base_main.and_then(|s| s.get("name"))),
            base_main,
            candidate_main,
        ));
    }
    let pairs = match_by_key(
        as_array(base.get("sequences")),
        as_array(candidate.get("sequences")),
        |item| as_string(item.get("name")),
    );
    for (name, base, candidate) in pairs {
        reports.push(diff_sequence(name, base, candidate));
    }
    // Drop fully-empty reports so callers see only touched sequences.
    reports.retain(|report| {
        !report.tracks_matched
            || report.tracks_added > 0
            || report.tracks_removed > 0
            || !report.clip_changes.is_empty()
            || report.markers_changed
            || report.graphics_changed
            || report.captions_changed
    });
    reports
}

fn diff_sequence(name: String, base: Option<&Value>, candidate: Option<&Value>) -> SequenceDiffReport {
    let mut report = SequenceDiffReport { name, ..Default::default() };
    let pairs = match_by_key(
        as_array(base.and_then(|s| s.get("tracks"))),
        as_array(candidate.and_then(|s| s.get("tracks"))),
        |item| item.get("id").and_then(Value::as_u64).unwrap_or(u64::MAX),
    );
    for (track_id, base, candidate) in pairs {
        if track_id == u64::MAX {
            // Track present on only one side: id unknown, report the side it
            // exists on without inventing clip-level details.
            if base.is_none() {
                report.tracks_added += 1;
            } else {
                report.tracks_removed += 1;
            }
            report.tracks_matched = true;
            continue;
        }
        if candidate.is_none() {
            report.tracks_removed += 1;
            report.tracks_matched = true;
            continue;
        }
        if base.is_none() {
            report.tracks_added += 1;
            report.tracks_matched = true;
            continue;
        }
        report.tracks_matched = true;
        let (base, candidate) = (base.unwrap(), candidate.unwrap());
        report
            .clip_changes
            .extend(diff_clip_list(track_id, base, candidate));
    }
    if flag_changed(base, candidate, "markers") {
        report.markers_changed = true;
    }
    if flag_changed(base, candidate, "graphics") {
        report.graphics_changed = true;
    }
    if flag_changed(base, candidate, "caption_tracks") {
        report.captions_changed = true;
    }
    report
}

fn diff_clip_list(track_id: u64, base: &Value, candidate: &Value) -> Vec<ClipChange> {
    let base = as_array(base.get("clips"));
    let candidate = as_array(candidate.get("clips"));
    let mut changes = Vec::new();
    let shared = base.len().min(candidate.len());
    for index in 0..shared {
        if base[index] == candidate[index] {
            continue;
        }
        let kind = if clip_only_moved(&base[index], &candidate[index]) {
            ClipChangeKind::Moved
        } else {
            ClipChangeKind::Changed
        };
        changes.push(ClipChange { track_id, clip_index: index, kind });
    }
    for index in shared..candidate.len() {
        changes.push(ClipChange { track_id, clip_index: index, kind: ClipChangeKind::Added });
    }
    for index in shared..base.len() {
        changes.push(ClipChange { track_id, clip_index: index, kind: ClipChangeKind::Removed });
    }
    changes
}

/// Only `timeline_start` differs: the edit moved the clip without touching it.
fn clip_only_moved(base: &Value, candidate: &Value) -> bool {
    let mut base_map = base.as_object().cloned().unwrap_or_default();
    let mut candidate_map = candidate.as_object().cloned().unwrap_or_default();
    base_map.remove("timeline_start");
    candidate_map.remove("timeline_start");
    base_map == candidate_map
}

fn flag_changed(base: Option<&Value>, candidate: Option<&Value>, key: &str) -> bool {
    as_array(base.and_then(|s| s.get(key))) != as_array(candidate.and_then(|s| s.get(key)))
}

fn as_array(value: Option<&Value>) -> &[Value] {
    value.and_then(Value::as_array).map(|slice| slice as &[Value]).unwrap_or(&[])
}

fn as_string(value: Option<&Value>) -> String {
    value
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned()
}

/// Pair array items between two snapshots by a derived key; unmatched items
/// get `None` on the missing side.
fn match_by_key<'a, K: PartialEq>(
    base: &'a [Value],
    candidate: &'a [Value],
    key_of: impl Fn(&Value) -> K,
) -> Vec<(K, Option<&'a Value>, Option<&'a Value>)> {
    let mut pairs = Vec::new();
    for item in base.iter() {
        let key = key_of(item);
        let candidate_index = candidate.iter().position(|other| key_of(other) == key);
        pairs.push((key, Some(item), candidate_index.map(|index| &candidate[index])));
    }
    for item in candidate.iter() {
        let key = key_of(item);
        if !base.iter().any(|other| key_of(other) == key) {
            pairs.push((key, None, Some(item)));
        }
    }
    pairs
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn project_with(clips: Value) -> Value {
        json!({
            "name": "Cut",
            "media": [
                {"id": 1, "name": "a.mov", "path": "/media/a.mov"},
                {"id": 2, "name": "b.mov", "path": "/media/b.mov"}
            ],
            "sequence": {
                "name": "Sequence 1",
                "tracks": [{"id": 7, "name": "V1", "clips": clips}],
                "markers": [],
                "graphics": [],
                "caption_tracks": []
            },
            "sequences": []
        })
    }

    #[test]
    fn identical_snapshots_report_identical() {
        let base = project_with(json!([
            {"media_id": 1, "timeline_start": 0, "duration": 100}
        ]));
        let report = diff_projects("fidget", &base, &base.clone());
        assert!(report.identical);
        assert!(report.media.added_ids.is_empty());
        assert!(report.sequences.is_empty());
    }

    #[test]
    fn media_add_remove_relink_is_reported() {
        let mut candidate = project_with(json!([]));
        candidate["media"] = json!([
            {"id": 1, "name": "a.mov", "path": "/moved/a.mov"},
            {"id": 3, "name": "c.mov", "path": "/media/c.mov"}
        ]);
        let report = diff_projects("fidget", &project_with(json!([])), &candidate);
        assert_eq!(report.media.added_ids, vec![3]);
        assert_eq!(report.media.removed_ids, vec![2]);
        assert_eq!(report.media.relinked.len(), 1);
        assert_eq!(report.media.relinked[0].from, "/media/a.mov");
        assert_eq!(report.media.relinked[0].to, "/moved/a.mov");
        assert!(!report.identical);
    }

    #[test]
    fn clip_moves_edits_and_lists_are_distinguished() {
        let base = project_with(json!([
            {"media_id": 1, "timeline_start": 0, "duration": 100},
            {"media_id": 2, "timeline_start": 100, "duration": 50}
        ]));
        let mut candidate = base.clone();
        candidate["sequence"]["tracks"][0]["clips"][0]["timeline_start"] = json!(75);
        candidate["sequence"]["tracks"][0]["clips"][1]["duration"] = json!(80);
        candidate["sequence"]["tracks"][0]["clips"].as_array_mut().unwrap()[0]["media_id"] =
            json!(2);
        let report = diff_projects("fidget", &base, &candidate);

        let track = &report.sequences[0];
        assert!(track.tracks_matched);
        let kinds: Vec<(usize, ClipChangeKind)> = track
            .clip_changes
            .iter()
            .map(|change| (change.clip_index, change.kind))
            .collect();
        // Slot 0 differs in timeline_start AND media_id -> changed, not moved.
        assert!(kinds.contains(&(0, ClipChangeKind::Changed)));
        // Slot 1 differs only in duration -> changed.
        assert!(kinds.contains(&(1, ClipChangeKind::Changed)));
    }

    #[test]
    fn pure_timeline_moves_report_as_moved() {
        let base = project_with(json!([
            {"media_id": 1, "timeline_start": 0, "duration": 100}
        ]));
        let mut candidate = base.clone();
        candidate["sequence"]["tracks"][0]["clips"][0]["timeline_start"] = json!(40);
        let report = diff_projects("fidget", &base, &candidate);
        assert_eq!(
            report.sequences[0].clip_changes[0].kind,
            ClipChangeKind::Moved
        );
    }

    #[test]
    fn marker_and_caption_changes_are_flagged() {
        let base = project_with(json!([]));
        let mut candidate = base.clone();
        candidate["sequence"]["markers"] = json!([{"id": 9, "name": "Q", "start_ticks": 0, "duration_ticks": 10}]);
        candidate["sequence"]["caption_tracks"] = json!([{"id": 1, "name": "Captions", "cues": [], "styles": []}]);
        let report = diff_projects("fidget", &base, &candidate);
        assert!(report.sequences[0].markers_changed);
        assert!(report.sequences[0].captions_changed);
    }

    #[test]
    fn added_sequences_and_tracks_are_counted() {
        let base = project_with(json!([]));
        let mut candidate = base.clone();
        candidate["sequences"] = json!([{
            "name": "B-roll",
            "tracks": [{"id": 3, "name": "V1", "clips": []}],
            "markers": [],
            "graphics": [],
            "caption_tracks": []
        }]);
        candidate["sequence"]["tracks"] = json!([
            {"id": 7, "name": "V1", "clips": []},
            {"id": 8, "name": "V2", "clips": []}
        ]);
        let report = diff_projects("fidget", &base, &candidate);
        assert_eq!(report.sequences.len(), 2);
        let b_roll = report
            .sequences
            .iter()
            .find(|report| report.name == "B-roll")
            .unwrap();
        assert_eq!(b_roll.tracks_added, 1);
        let main = report
            .sequences
            .iter()
            .find(|report| report.name == "Sequence 1")
            .unwrap();
        assert_eq!(main.tracks_added, 1);
    }

    #[test]
    fn branch_names_reject_empty_overlong_and_main() {
        assert_eq!(
            TimelineBranch::validate_name("  fidget "),
            Ok("fidget".to_owned())
        );
        assert!(TimelineBranch::validate_name("   ").is_err());
        assert!(TimelineBranch::validate_name("main").is_err());
        assert!(TimelineBranch::validate_name("MAIN").is_err());
        assert!(TimelineBranch::validate_name(&"x".repeat(121)).is_err());
        assert!(TimelineBranch::validate_name(&"x".repeat(120)).is_ok());
    }
}
