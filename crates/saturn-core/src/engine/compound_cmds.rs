use super::*;

impl EditorEngine {
    pub(super) fn dispatch_compound(
        &mut self,
        id: &str,
        params: Value,
    ) -> Option<Result<Value, DispatchError>> {
        match id {
            "timeline.create_compound_clip" => Some((|| {
                let params: CreateCompoundClipParams = parse_params(params)?;
                if params.clips.is_empty() {
                    return Err(DispatchError::Operation(
                        "No clips selected to create compound clip".into(),
                    ));
                }

                // Check for locked tracks among selected clips
                for clip_ref in &params.clips {
                    if let Some(track) = self
                        .document
                        .project
                        .sequence
                        .tracks
                        .iter()
                        .find(|t| t.id == clip_ref.track_id)
                    {
                        if track.locked {
                            return Err(DispatchError::Operation(format!(
                                "Track `{}` is locked",
                                track.name
                            )));
                        }
                    } else {
                        return Err(DispatchError::Operation(format!(
                            "Track id `{}` not found",
                            clip_ref.track_id
                        )));
                    }
                }

                // Find earliest start and latest end
                let mut earliest_start: i64 = i64::MAX;
                let mut latest_end: i64 = 0;
                let mut collected_clips: Vec<(u64, Clip)> = Vec::new();

                for clip_ref in &params.clips {
                    let track = self
                        .document
                        .project
                        .sequence
                        .tracks
                        .iter()
                        .find(|t| t.id == clip_ref.track_id)
                        .unwrap();
                    if clip_ref.clip_index >= track.clips.len() {
                        return Err(DispatchError::Operation(format!(
                            "Clip index {} out of bounds on track {}",
                            clip_ref.clip_index, track.id
                        )));
                    }
                    let clip = &track.clips[clip_ref.clip_index];
                    let start = clip.timeline_start.0;
                    let end = start.saturating_add(clip.duration.0);
                    if start < earliest_start {
                        earliest_start = start;
                    }
                    if end > latest_end {
                        latest_end = end;
                    }
                    collected_clips.push((track.id, clip.clone()));
                }

                if earliest_start > latest_end {
                    return Err(DispatchError::Operation("Invalid clip time ranges".into()));
                }

                let compound_duration = Tick(latest_end - earliest_start);
                let earliest_tick = Tick(earliest_start);

                self.checkpoint();

                // Group clips into nested tracks
                let mut nested_tracks: Vec<Track> = Vec::new();
                for (orig_track_id, clip) in collected_clips {
                    let orig_track = self
                        .document
                        .project
                        .sequence
                        .tracks
                        .iter()
                        .find(|t| t.id == orig_track_id)
                        .unwrap();
                    let nested_track = if let Some(nt) =
                        nested_tracks.iter_mut().find(|t| t.id == orig_track_id)
                    {
                        nt
                    } else {
                        nested_tracks.push(Track {
                            id: orig_track.id,
                            group_id: orig_track.group_id,
                            kind: orig_track.kind,
                            name: orig_track.name.clone(),
                            gain_db: orig_track.gain_db,
                            pan: orig_track.pan,
                            muted: orig_track.muted,
                            solo: orig_track.solo,
                            locked: false,
                            audio_processing: orig_track.audio_processing.clone(),
                            bus: orig_track.bus,
                            automation: TrackAutomation::default(),
                            clips: Vec::new(),
                            transitions: Vec::new(),
                        });
                        nested_tracks.last_mut().unwrap()
                    };

                    let mut nested_clip = clip.clone();
                    // Shift timeline start relative to earliest start so nested sequence begins at 0
                    nested_clip.timeline_start =
                        Tick(clip.timeline_start.0.saturating_sub(earliest_start));
                    nested_track.clips.push(nested_clip);
                }

                for track in &mut nested_tracks {
                    track.clips.sort_by_key(|c| c.timeline_start.0);
                }

                let new_sequence_id = self.document.project.next_id;
                self.document.project.next_id = new_sequence_id.saturating_add(1);

                let nested_sequence = Sequence {
                    id: new_sequence_id,
                    name: params.name.clone(),
                    track_groups: Vec::new(),
                    tracks: nested_tracks,
                    mark_in: None,
                    mark_out: None,
                    caption_tracks: Vec::new(),
                    graphics: Vec::new(),
                    markers: Vec::new(),
                    multicam_clips: Vec::new(),
                    buses: crate::project::default_audio_buses(),
                    node_graph: None,
                };

                let compound_media_id = self.document.project.next_id;
                self.document.project.next_id = compound_media_id.saturating_add(1);

                let compound_media_asset = MediaAsset {
                    id: compound_media_id,
                    name: params.name.clone(),
                    path: PathBuf::from(format!("compound://{}", compound_media_id)),
                    kind: MediaKind::CompoundSequence,
                    duration: Some(compound_duration),
                    width: Some(self.document.project.settings.width),
                    height: Some(self.document.project.settings.height),
                    nested_sequence: Some(nested_sequence.clone()),
                    ..Default::default()
                };

                self.document.project.media.push(compound_media_asset);
                self.document.project.sequences.push(nested_sequence);

                // Remove selected clips from active sequence (process indices descending per track)
                let mut by_track: std::collections::HashMap<u64, Vec<usize>> =
                    std::collections::HashMap::new();
                for clip_ref in &params.clips {
                    by_track
                        .entry(clip_ref.track_id)
                        .or_default()
                        .push(clip_ref.clip_index);
                }

                for (track_id, mut indices) in by_track {
                    indices.sort_unstable_by(|a, b| b.cmp(a));
                    indices.dedup();
                    if let Some(track) = self
                        .document
                        .project
                        .sequence
                        .tracks
                        .iter_mut()
                        .find(|t| t.id == track_id)
                    {
                        for idx in indices {
                            if idx < track.clips.len() {
                                track.clips.remove(idx);
                            }
                        }
                    }
                }

                let target_track_id = params.target_track_id.unwrap_or(params.clips[0].track_id);
                let target_track = self
                    .document
                    .project
                    .sequence
                    .tracks
                    .iter_mut()
                    .find(|t| t.id == target_track_id)
                    .ok_or_else(|| {
                        DispatchError::Operation(format!(
                            "Target track id {} not found",
                            target_track_id
                        ))
                    })?;

                let compound_clip = Clip {
                    media_id: compound_media_id,
                    timeline_start: earliest_tick,
                    source_in: Tick(0),
                    duration: compound_duration,
                    speed: 1.0,
                    group_id: None,
                    color: ColorAdjustments::default(),
                    effects: Vec::new(),
                    keyframes: Vec::new(),
                    multicam_id: None,
                    multicam_angle_id: None,
                    tracking: None,
                    node_graph: None,
                    speed_ramp: None,
                    motion_blur: None,
                    compound_sequence_id: Some(compound_media_id),
                };

                target_track.clips.push(compound_clip);
                target_track.clips.sort_by_key(|c| c.timeline_start.0);

                Ok(json!({
                    "compound_media_id": compound_media_id,
                    "sequence_id": new_sequence_id,
                    "name": params.name,
                    "timeline_start": earliest_start,
                    "duration": compound_duration.0,
                    "target_track_id": target_track_id,
                }))
            })()),
            "timeline.decompose_compound_clip" => Some((|| {
                let params: DecomposeCompoundClipParams = parse_params(params)?;
                let track = self
                    .document
                    .project
                    .sequence
                    .tracks
                    .iter()
                    .find(|t| t.id == params.track_id)
                    .ok_or_else(|| {
                        DispatchError::Operation(format!("Track id {} not found", params.track_id))
                    })?;

                if track.locked {
                    return Err(DispatchError::Operation(format!(
                        "Track `{}` is locked",
                        track.name
                    )));
                }

                if params.clip_index >= track.clips.len() {
                    return Err(DispatchError::Operation("Clip index out of bounds".into()));
                }

                let target_clip = track.clips[params.clip_index].clone();
                let compound_media = self
                    .document
                    .project
                    .media
                    .iter()
                    .find(|m| m.id == target_clip.media_id)
                    .ok_or_else(|| {
                        DispatchError::Operation("Compound media asset not found".into())
                    })?;

                if target_clip.compound_sequence_id.is_none()
                    && compound_media.kind != MediaKind::CompoundSequence
                {
                    return Err(DispatchError::Operation(
                        "Selected clip is not a compound clip".into(),
                    ));
                }

                let nested_seq = compound_media
                    .nested_sequence
                    .clone()
                    .or_else(|| {
                        target_clip.compound_sequence_id.and_then(|id| {
                            self.document
                                .project
                                .sequences
                                .iter()
                                .find(|s| s.id == id)
                                .cloned()
                        })
                    })
                    .ok_or_else(|| {
                        DispatchError::Operation(
                            "Nested sequence data not found for compound clip".into(),
                        )
                    })?;

                self.checkpoint();

                // Remove the compound clip from track
                let track_mut = self
                    .document
                    .project
                    .sequence
                    .tracks
                    .iter_mut()
                    .find(|t| t.id == params.track_id)
                    .unwrap();
                track_mut.clips.remove(params.clip_index);

                let time_offset = target_clip
                    .timeline_start
                    .0
                    .saturating_sub(target_clip.source_in.0);
                let mut unpacked_count = 0;

                for nested_track in nested_seq.tracks {
                    let dest_track = if let Some(t) = self
                        .document
                        .project
                        .sequence
                        .tracks
                        .iter_mut()
                        .find(|t| t.id == nested_track.id)
                    {
                        t
                    } else if let Some(t) = self
                        .document
                        .project
                        .sequence
                        .tracks
                        .iter_mut()
                        .find(|t| t.kind == nested_track.kind)
                    {
                        t
                    } else {
                        // Create matching track
                        let new_id = self.document.project.next_id;
                        self.document.project.next_id = new_id.saturating_add(1);
                        self.document.project.sequence.tracks.push(Track {
                            id: new_id,
                            group_id: None,
                            kind: nested_track.kind,
                            name: nested_track.name.clone(),
                            gain_db: nested_track.gain_db,
                            pan: nested_track.pan,
                            muted: nested_track.muted,
                            solo: nested_track.solo,
                            locked: false,
                            audio_processing: nested_track.audio_processing.clone(),
                            bus: nested_track.bus,
                            automation: TrackAutomation::default(),
                            clips: Vec::new(),
                            transitions: Vec::new(),
                        });
                        self.document.project.sequence.tracks.last_mut().unwrap()
                    };

                    for mut clip in nested_track.clips {
                        clip.timeline_start =
                            Tick(clip.timeline_start.0.saturating_add(time_offset));
                        dest_track.clips.push(clip);
                        unpacked_count += 1;
                    }
                    dest_track.clips.sort_by_key(|c| c.timeline_start.0);
                }

                Ok(json!({
                    "decomposed": true,
                    "unpacked_clips_count": unpacked_count,
                }))
            })()),
            _ => None,
        }
    }
}
