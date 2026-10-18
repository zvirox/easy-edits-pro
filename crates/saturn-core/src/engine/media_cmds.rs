use super::*;

impl EditorEngine {
    pub(super) fn dispatch_media(
        &mut self,
        id: &str,
        params: Value,
    ) -> Option<Result<Value, DispatchError>> {
        match id {
            "project.add_media" => Some((|| {
                let params: AddMediaParams = parse_params(params)?;
                if let Some(existing) = self
                    .document
                    .project
                    .media
                    .iter()
                    .find(|item| item.path == params.path)
                {
                    return Ok(json!({"media_id": existing.id, "already_present": true}));
                }
                self.checkpoint();
                let id = self.document.project.next_id;
                self.document.project.next_id = id.saturating_add(1);
                let item = MediaAsset {
                    id,
                    name: params.name,
                    path: params.path,
                    kind: params.kind,
                    duration: params.duration,
                    width: params.width,
                    height: params.height,
                    fps: params.fps,
                    audio_channels: params.audio_channels,
                    codec: params.codec,
                    bin_id: params.bin_id,
                    color_label: params.color_label,
                    tags: params.tags,
                    is_missing: false,
                    title: params.title,
                    waveform: params.waveform,
                    nested_sequence: None,
                };
                self.document.project.media.push(item);
                Ok(json!({"media_id": id}))
            })()),
            "project.update_media" => Some((|| {
                let params: UpdateMediaParams = parse_params(params)?;
                let Some(item) = self
                    .document
                    .project
                    .media
                    .iter_mut()
                    .find(|item| item.path == params.path)
                else {
                    return Ok(json!({"updated": false}));
                };
                item.kind = params.kind;
                item.duration = params.duration;
                item.width = params.width;
                item.height = params.height;
                if params.waveform.is_some() {
                    item.waveform = params.waveform;
                }
                Ok(json!({"updated": true, "media_id": item.id}))
            })()),
            "project.relink_media" => Some((|| {
                let params: RelinkMediaParams = parse_params(params)?;
                if !params.path.is_file() {
                    return Err(DispatchError::InvalidParameters(
                        "The replacement media file does not exist".into(),
                    ));
                }
                let Some(index) = self
                    .document
                    .project
                    .media
                    .iter()
                    .position(|item| item.id == params.media_id)
                else {
                    return Err(DispatchError::InvalidParameters(format!(
                        "Media item {} does not exist",
                        params.media_id
                    )));
                };
                self.checkpoint();
                let media = &mut self.document.project.media[index];
                media.path = params.path;
                Ok(json!({"relinked": true, "media_id": media.id, "path": media.path}))
            })()),
            "media.create_bin" => Some((|| {
                let params: CreateBinParams = parse_params(params)?;
                if params.name.trim().is_empty() {
                    return Err(DispatchError::InvalidParameters(
                        "Bin name cannot be empty".into(),
                    ));
                }
                self.checkpoint();
                let bin_id = self.document.project.next_id;
                self.document.project.next_id = bin_id.saturating_add(1);
                let bin = crate::project::MediaBin {
                    id: bin_id,
                    name: params.name.trim().to_string(),
                    parent_id: params.parent_id,
                    color: params.color.unwrap_or_else(|| "#38bdf8".into()),
                };
                self.document.project.bins.push(bin.clone());
                Ok(json!({
                    "created": true,
                    "bin": bin,
                }))
            })()),
            "media.rename_bin" => Some((|| {
                let params: RenameBinParams = parse_params(params)?;
                if params.name.trim().is_empty() {
                    return Err(DispatchError::InvalidParameters(
                        "Bin name cannot be empty".into(),
                    ));
                }
                let bin_idx = self
                    .document
                    .project
                    .bins
                    .iter()
                    .position(|b| b.id == params.bin_id)
                    .ok_or_else(|| {
                        DispatchError::InvalidParameters(format!("Bin {} not found", params.bin_id))
                    })?;
                self.checkpoint();
                self.document.project.bins[bin_idx].name = params.name.trim().to_string();
                Ok(json!({
                    "renamed": true,
                    "bin_id": params.bin_id,
                    "name": self.document.project.bins[bin_idx].name,
                }))
            })()),
            "media.delete_bin" => Some((|| {
                let params: DeleteBinParams = parse_params(params)?;
                if !self
                    .document
                    .project
                    .bins
                    .iter()
                    .any(|b| b.id == params.bin_id)
                {
                    return Err(DispatchError::InvalidParameters(format!(
                        "Bin {} not found",
                        params.bin_id
                    )));
                }
                self.checkpoint();
                self.document
                    .project
                    .bins
                    .retain(|b| b.id != params.bin_id && b.parent_id != Some(params.bin_id));
                for media in &mut self.document.project.media {
                    if media.bin_id == Some(params.bin_id) {
                        media.bin_id = None;
                    }
                }
                Ok(json!({
                    "deleted": true,
                    "bin_id": params.bin_id,
                }))
            })()),
            "media.move_to_bin" => Some((|| {
                let params: MoveToBinParams = parse_params(params)?;
                let media_idx = self
                    .document
                    .project
                    .media
                    .iter()
                    .position(|m| m.id == params.media_id)
                    .ok_or_else(|| {
                        DispatchError::InvalidParameters(format!(
                            "Media {} not found",
                            params.media_id
                        ))
                    })?;
                if let Some(target_bin) = params.bin_id {
                    if !self
                        .document
                        .project
                        .bins
                        .iter()
                        .any(|b| b.id == target_bin)
                    {
                        return Err(DispatchError::InvalidParameters(format!(
                            "Target bin {} not found",
                            target_bin
                        )));
                    }
                }
                self.checkpoint();
                self.document.project.media[media_idx].bin_id = params.bin_id;
                Ok(json!({
                    "moved": true,
                    "media_id": params.media_id,
                    "bin_id": params.bin_id,
                }))
            })()),
            "media.update_properties" => Some((|| {
                let params: UpdateMediaPropertiesParams = parse_params(params)?;
                let media_idx = self
                    .document
                    .project
                    .media
                    .iter()
                    .position(|m| m.id == params.media_id)
                    .ok_or_else(|| {
                        DispatchError::InvalidParameters(format!(
                            "Media {} not found",
                            params.media_id
                        ))
                    })?;
                self.checkpoint();
                let media = &mut self.document.project.media[media_idx];
                if let Some(name) = params.name {
                    if !name.trim().is_empty() {
                        media.name = name.trim().to_string();
                    }
                }
                if let Some(color_label) = params.color_label {
                    media.color_label = Some(color_label);
                }
                if let Some(tags) = params.tags {
                    media.tags = tags;
                }
                Ok(json!({
                    "updated": true,
                    "media_id": params.media_id,
                    "name": media.name,
                    "color_label": media.color_label,
                    "tags": media.tags,
                }))
            })()),
            "media.check_missing" => Some((|| {
                let mut missing_count = 0;
                let mut missing_ids = Vec::new();
                for media in &mut self.document.project.media {
                    if media.kind != MediaKind::Title && media.kind != MediaKind::CompoundSequence {
                        let is_missing = !media.path.exists();
                        media.is_missing = is_missing;
                        if is_missing {
                            missing_count += 1;
                            missing_ids.push(media.id);
                        }
                    }
                }
                Ok(json!({
                    "checked": true,
                    "missing_count": missing_count,
                    "missing_ids": missing_ids,
                }))
            })()),
            "media.list" => Some(
                serde_json::to_value(&self.document.project.media)
                    .map_err(|error| DispatchError::Operation(error.to_string())),
            ),
            _ => None,
        }
    }
}
