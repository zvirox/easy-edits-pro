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
                    comment: String::new(),
                    scene: String::new(),
                    take: String::new(),
                    is_missing: false,
                    source_mark_in: None,
                    source_mark_out: None,
                    proxy_path: None,
                    colorspace: None,
                    gamma: None,
                    file_size: None,
                    modified_ms: None,
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
                if params.comment.as_ref().is_some_and(|value| value.len() > 4000) {
                    return Err(DispatchError::InvalidParameters(
                        "Asset comment is limited to 4000 characters".into(),
                    ));
                }
                if params.scene.as_ref().is_some_and(|value| value.len() > 200)
                    || params.take.as_ref().is_some_and(|value| value.len() > 200)
                {
                    return Err(DispatchError::InvalidParameters(
                        "Scene and take are limited to 200 characters".into(),
                    ));
                }
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
                if let Some(comment) = params.comment {
                    media.comment = comment;
                }
                if let Some(scene) = params.scene {
                    media.scene = scene.trim().to_string();
                }
                if let Some(take) = params.take {
                    media.take = take.trim().to_string();
                }
                if let Some(colorspace) = params.colorspace {
                    media.colorspace = Some(colorspace);
                }
                if let Some(gamma) = params.gamma {
                    media.gamma = Some(gamma);
                }
                if let Some(audio_channels) = params.audio_channels {
                    media.audio_channels = Some(audio_channels);
                }
                Ok(json!({
                    "updated": true,
                    "media_id": params.media_id,
                    "name": media.name,
                    "color_label": media.color_label,
                    "tags": media.tags,
                    "comment": media.comment,
                    "scene": media.scene,
                    "take": media.take,
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
            // File-backed predicate shared by the media hygiene commands:
            // titles and compound sequences have no file to lose.
            "media.auto_relink" => Some((|| {
                let params: MediaAutoRelinkParams = parse_params(params)?;
                if params.search_dirs.is_empty() {
                    return Err(DispatchError::InvalidParameters(
                        "auto_relink needs at least one search directory".into(),
                    ));
                }
                let mut dirs: Vec<std::path::PathBuf> = Vec::new();
                for dir in &params.search_dirs {
                    let path = std::path::PathBuf::from(dir);
                    if !path.is_dir() {
                        return Err(DispatchError::InvalidParameters(format!(
                            "Search directory {dir} does not exist"
                        )));
                    }
                    dirs.push(path);
                }
                let mut matches: Vec<(u64, PathBuf)> = Vec::new();
                for media in &self.document.project.media {
                    if media.kind == MediaKind::Title
                        || media.kind == MediaKind::CompoundSequence
                        || media.path.exists()
                    {
                        continue;
                    }
                    if let Some(found) = dirs.iter().find_map(|dir| {
                        let candidate = dir.join(&media.path.file_name()?);
                        candidate.is_file().then(|| candidate)
                    }) {
                        matches.push((media.id, found));
                    }
                }
                if matches.is_empty() {
                    return Ok(json!({"relinked": 0, "still_missing": self
                        .document
                        .project
                        .media
                        .iter()
                        .filter(|media| {
                            media.kind != MediaKind::Title
                                && media.kind != MediaKind::CompoundSequence
                                && !media.path.exists()
                        })
                        .count()}));
                }
                self.checkpoint();
                for (id, found) in &matches {
                    let media = self
                        .document
                        .project
                        .media
                        .iter_mut()
                        .find(|media| media.id == *id)
                        .expect("collected from this list");
                    media.path = found.clone();
                    media.is_missing = false;
                }
                Ok(json!({
                    "relinked": matches.len(),
                    "matches": matches.iter().map(|(id, path)| json!({
                        "media_id": id, "path": path.to_string_lossy(),
                    })).collect::<Vec<_>>(),
                }))
            })()),
            "media.make_offline" => Some((|| {
                let params: MediaMakeOfflineParams = parse_params(params)?;
                if let Some(ids) = &params.media_ids {
                    for id in ids {
                        if !self.document.project.media.iter().any(|m| m.id == *id) {
                            return Err(DispatchError::InvalidParameters(format!(
                                "Media item {id} does not exist"
                            )));
                        }
                    }
                }
                let targets: Vec<u64> = self
                    .document
                    .project
                    .media
                    .iter()
                    .filter(|media| {
                        media.kind != MediaKind::Title
                            && media.kind != MediaKind::CompoundSequence
                            && !media.is_missing
                            && params
                                .media_ids
                                .as_ref()
                                .is_none_or(|ids| ids.contains(&media.id))
                    })
                    .map(|media| media.id)
                    .collect();
                if targets.is_empty() {
                    return Ok(json!({"offline": 0}));
                }
                self.checkpoint();
                for id in &targets {
                    let media = self
                        .document
                        .project
                        .media
                        .iter_mut()
                        .find(|media| media.id == *id)
                        .expect("collected above");
                    media.is_missing = true;
                }
                Ok(json!({"offline": targets.len()}))
            })()),
            "media.offline_all" => Some((|| {
                let targets: Vec<u64> = self
                    .document
                    .project
                    .media
                    .iter()
                    .filter(|media| {
                        media.kind != MediaKind::Title
                            && media.kind != MediaKind::CompoundSequence
                            && !media.is_missing
                    })
                    .map(|media| media.id)
                    .collect();
                if targets.is_empty() {
                    return Ok(json!({"offline": 0}));
                }
                self.checkpoint();
                for id in &targets {
                    let media = self
                        .document
                        .project
                        .media
                        .iter_mut()
                        .find(|media| media.id == *id)
                        .expect("collected above");
                    media.is_missing = true;
                }
                Ok(json!({"offline": targets.len()}))
            })()),
            "media.refresh_changed" => Some((|| {
                let params: MediaRefreshChangedParams = parse_params(params)?;
                if let Some(ids) = &params.media_ids {
                    for id in ids {
                        if !self.document.project.media.iter().any(|m| m.id == *id) {
                            return Err(DispatchError::InvalidParameters(format!(
                                "Media item {id} does not exist"
                            )));
                        }
                    }
                }
                let mut changed = Vec::new();
                let mut missing = Vec::new();
                let mut baselined = Vec::new();
                for media in self.document.project.media.iter_mut() {
                    if media.kind == MediaKind::Title
                        || media.kind == MediaKind::CompoundSequence
                    {
                        continue;
                    }
                    if params
                        .media_ids
                        .as_ref()
                        .is_some_and(|ids| !ids.contains(&media.id))
                    {
                        continue;
                    }
                    let Some(metadata) = std::fs::metadata(&media.path).ok() else {
                        media.is_missing = true;
                        missing.push(media.id);
                        continue;
                    };
                    media.is_missing = false;
                    let size = metadata.len();
                    let modified = metadata
                        .modified()
                        .ok()
                        .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
                        .map(|since| since.as_millis() as u64);
                    match (media.file_size, media.modified_ms) {
                        (Some(stored_size), Some(stored_modified))
                            if stored_size != size || Some(stored_modified) != modified =>
                        {
                            changed.push(media.id);
                        }
                        (None, _) | (_, None) => {
                            // First refresh records the baseline; not a change.
                            baselined.push(media.id);
                        }
                        _ => {}
                    }
                    media.file_size = Some(size);
                    media.modified_ms = modified;
                }
                Ok(json!({
                    "changed": changed,
                    "missing": missing,
                    "baselined": baselined,
                }))
            })()),
            "media.color_info" => Some((|| {
                let params: MediaColorInfoParams = parse_params(params)?;
                let media = self
                    .document
                    .project
                    .media
                    .iter()
                    .find(|media| media.id == params.media_id)
                    .ok_or_else(|| {
                        DispatchError::InvalidParameters(format!(
                            "Media item {} does not exist",
                            params.media_id
                        ))
                    })?;
                // Read-only: no checkpoint.
                Ok(json!({
                    "media_id": media.id,
                    "name": media.name,
                    "colorspace": media.colorspace,
                    "gamma": media.gamma,
                    "codec": media.codec,
                    "width": media.width,
                    "height": media.height,
                    "fps": media.fps,
                    "color_label": media.color_label,
                }))
            })()),
            "media.attach_proxy" => Some((|| {
                let params: MediaProxyAttachParams = parse_params(params)?;
                let proxy_path = PathBuf::from(&params.path);
                if !proxy_path.is_file() {
                    return Err(DispatchError::InvalidParameters(
                        "The proxy file does not exist".into(),
                    ));
                }
                let index = self
                    .document
                    .project
                    .media
                    .iter()
                    .position(|media| media.id == params.media_id)
                    .ok_or_else(|| {
                        DispatchError::InvalidParameters(format!(
                            "Media item {} does not exist",
                            params.media_id
                        ))
                    })?;
                self.checkpoint();
                let media = &mut self.document.project.media[index];
                media.proxy_path = Some(proxy_path.clone());
                Ok(json!({
                    "attached": true,
                    "media_id": media.id,
                    "proxy_path": proxy_path.to_string_lossy(),
                }))
            })()),
            "media.detach_proxy" => Some((|| {
                let params: MediaProxyDetachParams = parse_params(params)?;
                let media = self
                    .document
                    .project
                    .media
                    .iter()
                    .find(|media| media.id == params.media_id)
                    .ok_or_else(|| {
                        DispatchError::InvalidParameters(format!(
                            "Media item {} does not exist",
                            params.media_id
                        ))
                    })?;
                if media.proxy_path.is_none() {
                    return Err(DispatchError::InvalidParameters(format!(
                        "Media item {} has no proxy attached",
                        params.media_id
                    )));
                }
                self.checkpoint();
                let media = self
                    .document
                    .project
                    .media
                    .iter_mut()
                    .find(|media| media.id == params.media_id)
                    .expect("verified above");
                media.proxy_path = None;
                Ok(json!({"detached": true, "media_id": media.id}))
            })()),
            "media.toggle_proxies" => Some((|| {
                self.checkpoint();
                let settings = &mut self.document.project.settings;
                settings.use_proxies = !settings.use_proxies;
                Ok(json!({"use_proxies": settings.use_proxies}))
            })()),
            _ => None,
        }
    }
}
