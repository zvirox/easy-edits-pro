// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
use super::*;
use crate::asymmetric_trim::{AsymmetricTrimParams, plan};
use crate::audio_edit::{RenderedAudio, audio_clip, snapshot_digest};

impl EditorEngine {
    pub(super) fn dispatch_asymmetric_trim(
        &mut self,
        id: &str,
        mut params: Value,
    ) -> Option<Result<Value, DispatchError>> {
        if id != "timeline.asymmetric_trim" {
            return None;
        }
        Some((|| {
            let invalid = DispatchError::InvalidParameters;
            let object = params
                .as_object_mut()
                .ok_or_else(|| invalid("Expected split-edit parameters".into()))?;
            let expected = object.remove("expected_digest")
                .ok_or_else(|| invalid("Prepare this split edit with audio_process (operation asymmetric_trim) or the desktop command".into()))?;
            if expected.as_str()
                != Some(&snapshot_digest(&self.document).map_err(DispatchError::Operation)?)
            {
                return Err(invalid(
                    "Project changed during split-edit preparation; rerun the edit".into(),
                ));
            }
            let audio: RenderedAudio = serde_json::from_value(
                object
                    .remove("audio")
                    .ok_or_else(|| invalid("Prepared split-edit audio is required".into()))?,
            )
            .map_err(|e| invalid(e.to_string()))?;
            let p: AsymmetricTrimParams = parse_params(params)?;
            let plan = plan(&self.document, &p).map_err(invalid)?;
            if audio.duration != plan.duration || audio.channels != 2 {
                return Err(invalid(
                    "Prepared stereo audio must span the entire selected clip pair".into(),
                ));
            }
            let mut next = self.document.clone();
            let media_id = super::audio_edit_cmds::add_audio(&mut next, audio, "Split-edit audio")?;
            let track_id = next.project.next_id;
            next.project.next_id = track_id
                .checked_add(1)
                .ok_or_else(|| invalid("Project id space exhausted".into()))?;
            let picture = &mut next.project.sequence.tracks[plan.track_index];
            let mut sound = picture.clone();
            sound.id = track_id;
            sound.kind = TrackKind::Audio;
            sound.name = format!("{} - split-edit audio", picture.name);
            sound.clips = vec![audio_clip(media_id, plan.start, plan.duration)];
            picture.clips[plan.left_index] = plan.left_picture;
            picture.clips[p.clip_index] = plan.right_picture;
            next.project.sequence.tracks.push(sound);
            next.validate().map_err(DispatchError::Operation)?;
            self.checkpoint();
            self.document = next;
            Ok(
                json!({"video_cut": plan.video_cut, "audio_cut": plan.audio_cut,
                "fade_ticks": p.fade_ticks, "audio_track_id": track_id, "media_id": media_id}),
            )
        })())
    }
}
