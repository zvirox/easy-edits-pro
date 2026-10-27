use super::*;

impl EditorEngine {
    fn allocate_id(&mut self) -> u64 {
        let id = self.document.project.next_id;
        self.document.project.next_id = id.saturating_add(1);
        id
    }

    /// All sequence names, optionally ignoring one sequence so renames and
    /// duplicates can keep their own name.
    fn sequence_names(&self, exclude: Option<u64>) -> Vec<String> {
        let mut names = Vec::new();
        if Some(self.document.project.sequence.id) != exclude {
            names.push(self.document.project.sequence.name.clone());
        }
        for sequence in &self.document.project.sequences {
            if Some(sequence.id) != exclude {
                names.push(sequence.name.clone());
            }
        }
        names
    }

    fn unique_sequence_name(&self, requested: &str, exclude: Option<u64>) -> String {
        let base = requested.trim();
        let base = if base.is_empty() { "Sequence" } else { base };
        let names = self.sequence_names(exclude);
        if !names.iter().any(|name| name.eq_ignore_ascii_case(base)) {
            return base.to_owned();
        }
        for suffix in 2u32.. {
            let candidate = format!("{base} {suffix}");
            if !names
                .iter()
                .any(|name| name.eq_ignore_ascii_case(&candidate))
            {
                return candidate;
            }
        }
        unreachable!("unbounded sequence name suffix")
    }

    fn blank_sequence(&mut self, name: String) -> Sequence {
        let video_id = self.allocate_id();
        let audio_id = self.allocate_id();
        Sequence {
            id: self.allocate_id(),
            name,
            track_groups: Vec::new(),
            tracks: vec![
                Track {
                    id: video_id,
                    group_id: None,
                    kind: TrackKind::Video,
                    name: "V1".into(),
                    gain_db: 0.0,
                    pan: 0.0,
                    muted: false,
                    solo: false,
                    locked: false,
                    audio_processing: TrackAudioProcessing::default(),
                    bus: BusTarget::Master,
                    automation: TrackAutomation::default(),
                    clips: Vec::new(),
                    transitions: Vec::new(),
                },
                Track {
                    id: audio_id,
                    group_id: None,
                    kind: TrackKind::Audio,
                    name: "A1".into(),
                    gain_db: 0.0,
                    pan: 0.0,
                    muted: false,
                    solo: false,
                    locked: false,
                    audio_processing: TrackAudioProcessing::default(),
                    bus: BusTarget::Master,
                    automation: TrackAutomation::default(),
                    clips: Vec::new(),
                    transitions: Vec::new(),
                },
            ],
            mark_in: None,
            mark_out: None,
            caption_tracks: Vec::new(),
                    graphics: Vec::new(),
            markers: Vec::new(),
            multicam_clips: Vec::new(),
            buses: crate::project::default_audio_buses(),
            node_graph: None,
        }
    }

    fn sequence_is_compound(&self, sequence_id: u64) -> bool {
        self.document.project.media.iter().any(|media| {
            media
                .nested_sequence
                .as_ref()
                .is_some_and(|sequence| sequence.id == sequence_id)
        })
    }

    fn sequence_clone(&self, sequence_id: u64) -> Option<Sequence> {
        if self.document.project.sequence.id == sequence_id {
            return Some(self.document.project.sequence.clone());
        }
        self.document
            .project
            .sequences
            .iter()
            .find(|sequence| sequence.id == sequence_id)
            .cloned()
    }

    fn target_sequence_id(&self, requested: Option<u64>) -> u64 {
        requested.unwrap_or(self.document.project.sequence.id)
    }

    pub(super) fn dispatch_sequence(
        &mut self,
        id: &str,
        params: Value,
    ) -> Option<Result<Value, DispatchError>> {
        match id {
            "sequence.create" => Some((|| {
                let params: CreateSequenceParams = parse_params(params)?;
                let fallback = format!("Sequence {}", 1 + self.document.project.sequences.len());
                let name =
                    self.unique_sequence_name(params.name.as_deref().unwrap_or(&fallback), None);

                self.checkpoint();
                let sequence = self.blank_sequence(name.clone());
                let sequence_id = sequence.id;
                let previous = std::mem::replace(&mut self.document.project.sequence, sequence);
                self.document.project.sequences.push(previous);
                self.document.view.active_sequence = Some(name.clone());

                Ok(json!({
                    "created": true,
                    "sequence_id": sequence_id,
                    "name": name,
                    "activated": true,
                }))
            })()),
            "sequence.duplicate" => Some((|| {
                let params: DuplicateSequenceParams = parse_params(params)?;
                let source_id = self.target_sequence_id(params.sequence_id);
                let source = self.sequence_clone(source_id).ok_or_else(|| {
                    DispatchError::Operation(format!("Sequence id {source_id} not found"))
                })?;
                let requested = params
                    .name
                    .unwrap_or_else(|| format!("{} copy", source.name));
                let name = self.unique_sequence_name(&requested, None);

                self.checkpoint();
                let sequence_id = self.allocate_id();
                let mut copy = source;
                copy.id = sequence_id;
                copy.name = name.clone();

                let activated = self.document.project.sequence.id == source_id;
                if activated {
                    let previous = std::mem::replace(&mut self.document.project.sequence, copy);
                    self.document.project.sequences.push(previous);
                    self.document.view.active_sequence = Some(name.clone());
                } else {
                    self.document.project.sequences.push(copy);
                }

                Ok(json!({
                    "created": true,
                    "duplicated": true,
                    "sequence_id": sequence_id,
                    "source_sequence_id": source_id,
                    "name": name,
                    "activated": activated,
                }))
            })()),
            "sequence.rename" => Some((|| {
                let params: RenameSequenceParams = parse_params(params)?;
                let sequence_id = self.target_sequence_id(params.sequence_id);
                if params.name.trim().is_empty() {
                    return Err(DispatchError::InvalidParameters(
                        "Sequence name cannot be empty".into(),
                    ));
                }
                let name = self.unique_sequence_name(&params.name, Some(sequence_id));

                self.checkpoint();
                if self.document.project.sequence.id == sequence_id {
                    self.document.project.sequence.name = name.clone();
                    self.document.view.active_sequence = Some(name.clone());
                } else {
                    let sequence = self
                        .document
                        .project
                        .sequences
                        .iter_mut()
                        .find(|sequence| sequence.id == sequence_id)
                        .ok_or_else(|| {
                            DispatchError::Operation(format!("Sequence id {sequence_id} not found"))
                        })?;
                    sequence.name = name.clone();
                }

                Ok(json!({
                    "renamed": true,
                    "sequence_id": sequence_id,
                    "name": name,
                }))
            })()),
            "sequence.delete" => Some((|| {
                let params: DeleteSequenceParams = parse_params(params)?;
                let sequence_id = self.target_sequence_id(params.sequence_id);
                if self.sequence_is_compound(sequence_id) {
                    return Err(DispatchError::Operation(
                        "A compound clip's nested sequence cannot be deleted".into(),
                    ));
                }

                let total = 1 + self.document.project.sequences.len();
                if total <= 1 {
                    return Err(DispatchError::Operation(
                        "A project must keep at least one sequence".into(),
                    ));
                }

                let active = self.document.project.sequence.id == sequence_id;
                if active {
                    let candidates: Vec<u64> = self
                        .document
                        .project
                        .sequences
                        .iter()
                        .map(|sequence| sequence.id)
                        .collect();
                    let replacement_id = candidates
                        .into_iter()
                        .find(|candidate| !self.sequence_is_compound(*candidate))
                        .ok_or_else(|| {
                            DispatchError::Operation(
                                "Create another sequence before deleting this one".into(),
                            )
                        })?;
                    let replacement_index = self
                        .document
                        .project
                        .sequences
                        .iter()
                        .position(|sequence| sequence.id == replacement_id)
                        .expect("replacement sequence index");
                    self.checkpoint();
                    let replacement = self.document.project.sequences.remove(replacement_index);
                    let replacement_name = replacement.name.clone();
                    let replacement_id = replacement.id;
                    self.document.project.sequence = replacement;
                    self.document.view.active_sequence = Some(replacement_name.clone());
                    return Ok(json!({
                        "deleted": true,
                        "sequence_id": sequence_id,
                        "active_sequence_id": replacement_id,
                        "active_sequence_name": replacement_name,
                    }));
                }

                if !self
                    .document
                    .project
                    .sequences
                    .iter()
                    .any(|sequence| sequence.id == sequence_id)
                {
                    return Err(DispatchError::Operation(format!(
                        "Sequence id {sequence_id} not found"
                    )));
                }
                self.checkpoint();
                self.document
                    .project
                    .sequences
                    .retain(|sequence| sequence.id != sequence_id);
                Ok(json!({
                    "deleted": true,
                    "sequence_id": sequence_id,
                }))
            })()),
            "sequence.lift" => Some((|| {
                let params: TimeRangeListParams = parse_params(params)?;
                if params.ranges.is_empty() {
                    return Err(DispatchError::InvalidParameters(
                        "Provide one or more lift ranges".into(),
                    ));
                }
                self.checkpoint();
                let report = crate::timeline_edits::lift_time_ranges(
                    &mut self.document.project.sequence,
                    &params.ranges,
                )
                .map_err(DispatchError::Operation)?;
                Ok(serde_json::to_value(&report).unwrap_or(json!({"lifted": true})))
            })()),
            "sequence.extract" => Some((|| {
                let params: TimeRangeListParams = parse_params(params)?;
                if params.ranges.is_empty() {
                    return Err(DispatchError::InvalidParameters(
                        "Provide one or more extract ranges".into(),
                    ));
                }
                self.checkpoint();
                let report = crate::speech::ripple_delete_time_ranges(
                    &mut self.document.project.sequence,
                    &params.ranges,
                )
                .map_err(DispatchError::Operation)?;
                Ok(serde_json::to_value(&report).unwrap_or(json!({"extracted": true})))
            })()),
            "sequence.add_edit_all_tracks" => Some((|| {
                let params: SequenceAddEditAllTracksParams = parse_params(params)?;
                if params.at.0 <= 0 {
                    return Err(DispatchError::InvalidParameters(
                        "Add Edit needs a playhead position past time zero".into(),
                    ));
                }
                // Collect (track_index, clip_index) for every unlocked clip
                // spanning `at`; descending clip index keeps the indices
                // valid while earlier splits insert right-neighbours.
                let mut targets = Vec::new();
                for (track_index, track) in
                    self.document.project.sequence.tracks.iter().enumerate()
                {
                    if track.locked {
                        continue;
                    }
                    for (clip_index, clip) in track.clips.iter().enumerate() {
                        let start = clip.timeline_start.0;
                        if start < params.at.0 && params.at.0 < start + clip.duration.0 {
                            targets.push((track_index, clip_index));
                        }
                    }
                }
                if targets.is_empty() {
                    return Err(DispatchError::Operation(
                        "No clips under the playhead to add edits to".into(),
                    ));
                }
                targets.sort_by_key(|&(_, clip_index)| std::cmp::Reverse(clip_index));
                let added = targets.len();
                self.checkpoint();
                for (track_index, clip_index) in targets {
                    apply_clip_split(&mut self.document, track_index, clip_index, params.at)?;
                }
                Ok(json!({"added_edits": added, "at": params.at.0}))
            })()),
            "sequence.settings" => Some((|| {
                let params: SequenceSettingsParams = parse_params(params)?;
                if let Some(width) = params.width {
                    if !(16..=8192).contains(&width) {
                        return Err(DispatchError::InvalidParameters(
                            "Sequence width must be between 16 and 8192".into(),
                        ));
                    }
                }
                if let Some(height) = params.height {
                    if !(16..=8192).contains(&height) {
                        return Err(DispatchError::InvalidParameters(
                            "Sequence height must be between 16 and 8192".into(),
                        ));
                    }
                }
                if let Some(frame_rate) = params.frame_rate {
                    if frame_rate.numerator == 0 || frame_rate.denominator == 0 {
                        return Err(DispatchError::InvalidParameters(
                            "Frame rate numerator and denominator must be positive".into(),
                        ));
                    }
                }
                self.checkpoint();
                let settings = &mut self.document.project.settings;
                if let Some(width) = params.width {
                    settings.width = width;
                }
                if let Some(height) = params.height {
                    settings.height = height;
                }
                if let Some(frame_rate) = params.frame_rate {
                    settings.frame_rate = frame_rate;
                }
                Ok(json!({
                    "width": settings.width,
                    "height": settings.height,
                    "frame_rate": {
                        "numerator": settings.frame_rate.numerator,
                        "denominator": settings.frame_rate.denominator,
                    },
                }))
            })()),
            "sequence.inspect" => Some((|| {
                let sequence = &self.document.project.sequence;
                let clip_total: usize = sequence.tracks.iter().map(|t| t.clips.len()).sum();
                let last_tick = sequence
                    .tracks
                    .iter()
                    .flat_map(|track| track.clips.iter())
                    .map(|clip| clip.timeline_start.0 + clip.duration.0)
                    .max()
                    .unwrap_or(0);
                let marks = usize::from(sequence.mark_in.is_some())
                    + usize::from(sequence.mark_out.is_some());
                Ok(json!({
                    "sequence_id": sequence.id,
                    "name": sequence.name,
                    "tracks": sequence.tracks.len(),
                    "clips": clip_total,
                    "duration": last_tick,
                    "marks": marks,
                    "markers": sequence.markers.len(),
                    "compound": self.sequence_is_compound(sequence.id),
                }))
            })()),
            "sequence.close" => Some((|| {
                let total = 1 + self.document.project.sequences.len();
                if total <= 1 {
                    return Err(DispatchError::Operation(
                        "The last remaining sequence cannot be closed".into(),
                    ));
                }
                // Closing parks the active sequence in the project list and
                // activates the first stored one; nothing is deleted.
                self.checkpoint();
                let previous = std::mem::replace(
                    &mut self.document.project.sequence,
                    self.document.project.sequences.remove(0),
                );
                self.document.project.sequences.push(previous);
                let name = self.document.project.sequence.name.clone();
                self.document.view.active_sequence = Some(name.clone());
                Ok(json!({
                    "closed": true,
                    "active_sequence_id": self.document.project.sequence.id,
                    "active_sequence_name": name,
                }))
            })()),
            "sequence.reveal_nested" => Some((|| {
                let params: RevealNestedSequenceParams = parse_params(params)?;
                let track_index = find_track_index(&self.document, params.track_id)?;
                let clip = find_clip(&self.document, track_index, params.clip_index)?;
                let media_id = clip
                    .compound_sequence_id
                    .ok_or_else(|| {
                        DispatchError::InvalidParameters(
                            "Timeline clip is not a compound/nested clip".into(),
                        )
                    })?;
                let nested = self
                    .document
                    .project
                    .media
                    .iter()
                    .find(|media| media.id == media_id)
                    .and_then(|media| media.nested_sequence.as_ref())
                    .ok_or_else(|| {
                        DispatchError::Operation(
                            "The compound clip does not carry a nested sequence".into(),
                        )
                    })?;
                Ok(json!({
                    "sequence_id": nested.id,
                    "name": nested.name,
                    "tracks": nested.tracks.len(),
                    "clips": nested.tracks.iter().map(|t| t.clips.len()).sum::<usize>(),
                }))
            })()),
            _ => None,
        }
    }
}
