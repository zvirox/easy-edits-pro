//! Timeline track-targeting family: the FC-style targeted video/audio track
//! sets that drive trims, overwrites, and the mixer.
//!
//! Targeting is editor state (not document state), like the trim family's
//! `TrimState` — it survives undoes of document edits and is never saved
//! into the project file. Ordinals are 1-based within each track kind, in
//! sequence track order (`V1` = the first video track, `A1` = the first
//! audio track). The three targeted-flag toggles are document mutations
//! (checkpoint) applying the existing `Track.muted`/`Track.solo` flags to
//! the targeted tracks.

use super::*;

/// Editor-held state for the timeline targeting family.
#[derive(Clone)]
pub(in crate::engine) struct TargetingState {
    /// 1-based video-track ordinals, sorted and deduplicated.
    video: Vec<u64>,
    /// 1-based audio-track ordinals, sorted and deduplicated.
    audio: Vec<u64>,
}

impl TargetingState {
    pub(in crate::engine) fn new() -> Self {
        Self {
            video: vec![1],
            audio: vec![1],
        }
    }

    fn get(&self, kind: TrackKind) -> &[u64] {
        match kind {
            TrackKind::Video => &self.video,
            TrackKind::Audio => &self.audio,
        }
    }

    fn get_mut(&mut self, kind: TrackKind) -> &mut Vec<u64> {
        match kind {
            TrackKind::Video => &mut self.video,
            TrackKind::Audio => &mut self.audio,
        }
    }

    fn set(&mut self, kind: TrackKind, ordinals: Vec<u64>) {
        let mut list = ordinals;
        list.sort_unstable();
        list.dedup();
        *self.get_mut(kind) = list;
    }
}

/// The count of tracks of one kind, which bounds valid ordinals.
fn kind_track_count(document: &ProjectDocument, kind: TrackKind) -> usize {
    document
        .project
        .sequence
        .tracks
        .iter()
        .filter(|track| track.kind == kind)
        .count()
}

/// Sequence indices of the targeted tracks of one kind, in document order.
fn kind_track_positions(
    document: &ProjectDocument,
    kind: TrackKind,
    ordinals: &[u64],
) -> Vec<usize> {
    let mut positions = Vec::new();
    let mut ordinal = 0u64;
    for (index, track) in document.project.sequence.tracks.iter().enumerate() {
        if track.kind == kind {
            ordinal = ordinal.saturating_add(1);
            if ordinals.contains(&ordinal) {
                positions.push(index);
            }
        }
    }
    positions
}

fn kind_name(kind: TrackKind) -> &'static str {
    match kind {
        TrackKind::Video => "video",
        TrackKind::Audio => "audio",
    }
}

/// The track flag flipped by the targeted-flag toggles.
#[derive(Clone, Copy)]
enum TargetedFlag {
    Muted,
    Solo,
}

/// Targeting feedback shared by the state commands.
fn targeting_payload(engine: &EditorEngine) -> Value {
    json!({
        "video_targets": engine.targeting.video.clone(),
        "audio_targets": engine.targeting.audio.clone(),
    })
}

impl EditorEngine {
    pub(super) fn dispatch_timeline_targeting(
        &mut self,
        id: &str,
        params: Value,
    ) -> Option<Result<Value, DispatchError>> {
        if let Some(rest) = id.strip_prefix("timeline.toggleTarget") {
            return Some(self.toggle_target(rest));
        }
        match id {
            "timeline.setTargeting" => Some(self.set_targeting(if params.is_null() {
                json!({})
            } else {
                params
            })),
            "timeline.moveVideoTargetsUp" => Some(self.move_targets(TrackKind::Video, 1)),
            "timeline.moveVideoTargetsDown" => Some(self.move_targets(TrackKind::Video, -1)),
            "timeline.moveAudioTargetsUp" => Some(self.move_targets(TrackKind::Audio, 1)),
            "timeline.moveAudioTargetsDown" => Some(self.move_targets(TrackKind::Audio, -1)),
            // FC's source patching shares the targeting state in EEP; both
            // toggle-all commands flip the same per-kind set.
            "timeline.toggleAllVideoTargets" | "timeline.toggleAllSourceVideo" => {
                Some(self.toggle_all_targets(TrackKind::Video))
            }
            "timeline.toggleAllAudioTargets" | "timeline.toggleAllSourceAudio" => {
                Some(self.toggle_all_targets(TrackKind::Audio))
            }
            // Track output for video tracks is represented by the muted flag
            // (output off = muted), matching the mixer's mute semantics.
            "timeline.toggleMuteTargetedAudio" => {
                Some(self.toggle_targeted_flag(TrackKind::Audio, TargetedFlag::Muted))
            }
            "timeline.toggleSoloTargetedAudio" => {
                Some(self.toggle_targeted_flag(TrackKind::Audio, TargetedFlag::Solo))
            }
            "timeline.toggleOutputTargetedVideo" => {
                Some(self.toggle_targeted_flag(TrackKind::Video, TargetedFlag::Muted))
            }
            _ => None,
        }
    }

    /// `timeline.toggleTargetV<n>` / `timeline.toggleTargetA<n>`.
    fn toggle_target(&mut self, rest: &str) -> Result<Value, DispatchError> {
        let (kind, ordinal) = if let Some(ordinal) = rest.strip_prefix('V') {
            (TrackKind::Video, ordinal)
        } else if let Some(ordinal) = rest.strip_prefix('A') {
            (TrackKind::Audio, ordinal)
        } else {
            return Err(DispatchError::InvalidParameters(format!(
                "Unknown targeting toggle 'timeline.toggleTarget{rest}'"
            )));
        };
        let ordinal: u64 = ordinal
            .parse()
            .map_err(|_| {
                DispatchError::InvalidParameters(format!(
                    "Unknown targeting toggle 'timeline.toggleTarget{rest}'"
                ))
            })?;
        if ordinal == 0 {
            return Err(DispatchError::InvalidParameters(
                "Target ordinals start at 1".into(),
            ));
        }
        let count = kind_track_count(&self.document, kind) as u64;
        if ordinal > count {
            return Err(DispatchError::InvalidParameters(format!(
                "No {} track {} in this sequence",
                kind_name(kind),
                ordinal
            )));
        }
        let state = &mut self.targeting;
        let list = state.get_mut(kind);
        if let Some(position) = list.iter().position(|item| *item == ordinal) {
            list.remove(position);
        } else {
            list.push(ordinal);
        }
        Ok(targeting_payload(self))
    }

    /// `timeline.setTargeting` — explicit 1-based ordinal lists. Called with
    /// no fields (menu item or CLI status read) it reports the current sets.
    fn set_targeting(&mut self, params: Value) -> Result<Value, DispatchError> {
        let params: SetTargetingParams = parse_params(params)?;
        for (kind, list) in [
            (TrackKind::Video, params.video.clone()),
            (TrackKind::Audio, params.audio.clone()),
        ] {
            if let Some(list) = list {
                let count = kind_track_count(&self.document, kind) as u64;
                for ordinal in list {
                    if ordinal == 0 || ordinal > count {
                        return Err(DispatchError::InvalidParameters(format!(
                            "No {} track {} in this sequence",
                            kind_name(kind),
                            ordinal
                        )));
                    }
                }
            }
        }
        if let Some(video) = params.video {
            self.targeting.set(TrackKind::Video, video);
        }
        if let Some(audio) = params.audio {
            self.targeting.set(TrackKind::Audio, audio);
        }
        Ok(targeting_payload(self))
    }

    /// Shift every targeted ordinal by `delta`, clamped to `1..=count`.
    fn move_targets(&mut self, kind: TrackKind, delta: i64) -> Result<Value, DispatchError> {
        let count = kind_track_count(&self.document, kind) as u64;
        let moved: Vec<u64> = self
            .targeting
            .get(kind)
            .iter()
            .map(|ordinal| {
                let shifted = *ordinal as i64 + delta;
                shifted.clamp(1, count as i64) as u64
            })
            .collect();
        self.targeting.set(kind, moved);
        Ok(targeting_payload(self))
    }

    /// All-on/all-off flip: when every track of the kind is targeted, clear
    /// the set; otherwise target them all.
    fn toggle_all_targets(&mut self, kind: TrackKind) -> Result<Value, DispatchError> {
        let count = kind_track_count(&self.document, kind) as u64;
        if self.targeting.get(kind).len() == count as usize {
            self.targeting.set(kind, Vec::new());
        } else {
            self.targeting
                .set(kind, (1..=count).collect());
        }
        Ok(targeting_payload(self))
    }

    /// Document mutation: flip one track flag (muted/solo) on every targeted
    /// track of the kind. Empty targeting is an error, matching the FC
    /// behaviour of the toggle applying to the targeted set.
    fn toggle_targeted_flag(
        &mut self,
        kind: TrackKind,
        flag: TargetedFlag,
    ) -> Result<Value, DispatchError> {
        let ordinals = self.targeting.get(kind).to_vec();
        if ordinals.is_empty() {
            return Err(DispatchError::Operation(format!(
                "No targeted {} tracks",
                kind_name(kind)
            )));
        }
        let positions = kind_track_positions(&self.document, kind, &ordinals);
        self.checkpoint();
        let mut updated = 0;
        for index in positions {
            let track = &mut self.document.project.sequence.tracks[index];
            match flag {
                TargetedFlag::Muted => track.muted = !track.muted,
                TargetedFlag::Solo => track.solo = !track.solo,
            }
            updated += 1;
        }
        let mut payload = targeting_payload(self);
        if let Some(object) = payload.as_object_mut() {
            object.insert("updated".into(), json!(updated));
        }
        Ok(payload)
    }
}
