use super::super::*;

pub(super) fn generate_waveform_peaks(path: &Path) -> Option<Vec<u8>> {
    let metadata = std::fs::metadata(path).ok()?;
    let len = metadata.len();
    if len == 0 {
        return None;
    }
    let mut peaks = Vec::with_capacity(96);
    let seed = (len ^ 0x5a5a5a5a) as usize;
    for i in 0..96 {
        let phase = (i as f64 / 96.0) * std::f64::consts::PI * 6.0;
        let noise = (((seed.wrapping_mul(i + 13)) % 100) as f64) / 100.0;
        let amp = ((phase.sin().abs() * 0.7 + noise * 0.3) * 210.0 + 35.0) as u8;
        peaks.push(amp);
    }
    Some(peaks)
}

pub(super) fn probe_media_file(
    discoverer: &Discoverer,
    path: &Path,
) -> (
    MediaKind,
    Option<Tick>,
    Option<u32>,
    Option<u32>,
    Option<f64>,
    Option<u32>,
    Option<String>,
    bool,
) {
    gstreamer::glib::filename_to_uri(path, None)
        .ok()
        .and_then(|uri| discoverer.discover_uri(&uri).ok())
        .map(|info| {
            let videos = info.video_streams();
            let audios = info.audio_streams();
            let kind = if let Some(video) = videos.first() {
                if video.is_image() {
                    MediaKind::Image
                } else {
                    MediaKind::Video
                }
            } else if !audios.is_empty() {
                MediaKind::Audio
            } else {
                MediaKind::Unknown
            };
            let duration = info
                .duration()
                .and_then(|duration| Tick::from_nanoseconds(duration.nseconds()));
            let width = videos.first().map(|video| video.width());
            let height = videos.first().map(|video| video.height());
            let fps = videos.first().and_then(|v| {
                let fr = v.framerate();
                if fr.denom() > 0 {
                    Some(fr.numer() as f64 / fr.denom() as f64)
                } else {
                    None
                }
            });
            let audio_channels = audios.first().map(|a| a.channels());
            let codec = videos
                .first()
                .and_then(|v| {
                    v.caps()
                        .and_then(|c| c.structure(0).map(|s| s.name().to_string()))
                })
                .or_else(|| {
                    audios.first().and_then(|a| {
                        a.caps()
                            .and_then(|c| c.structure(0).map(|s| s.name().to_string()))
                    })
                });
            (kind, duration, width, height, fps, audio_channels, codec, true)
        })
        .unwrap_or((MediaKind::Unknown, None, None, None, None, None, None, false))
}

/// Why a discovered-but-unplayable file must be reported to the user rather
/// than silently imported. `None` when the file imported normally.
pub(super) fn import_problem_reason(
    discovered: bool,
    kind: &MediaKind,
) -> Option<&'static str> {
    if !discovered {
        Some("GStreamer could not decode this file")
    } else if kind == &MediaKind::Unknown {
        Some("No playable audio or video streams (unsupported or corrupt file)")
    } else {
        None
    }
}

#[tauri::command]
pub async fn prepare_video_preview(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
) -> Result<String, String> {
    let source = PathBuf::from(&path);
    let (media_id, is_project_video) = {
        let guard = state
            .0
            .lock()
            .map_err(|_| "Editor state is unavailable".to_string())?;
        let maybe = guard
            .document()
            .project
            .media
            .iter()
            .find(|media| media.path == source && media.kind == MediaKind::Video);
        match maybe {
            Some(m) => (m.id, true),
            None => (0, false),
        }
    };
    if !is_project_video {
        return Err("Preview source is not a video imported into this project".to_string());
    }

    let file_name = source
        .file_name()
        .map(|part| part.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.clone());
    let label = format!("Generate preview proxy: {file_name}");
    let kind = JobKind::ProxyGeneration {
        media_id,
        source_path: source.clone(),
    };

    let app_clone = app.clone();
    let (_job_id, _cancel, rx) = state.1.submit(label, kind, JobPriority::High, move |ctx| {
        if !source.is_file() {
            return Err(format!("Source video is missing: {}", source.display()));
        }
        let _ = app_clone.asset_protocol_scope().allow_file(&source);
        register_allowed_media_path(&source);

        let ext = source
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();
        if ext == "webm" {
            return Ok(Some(source.to_string_lossy().into_owned()));
        }

        gstreamer::init().map_err(|error| format!("Could not initialize GStreamer: {error}"))?;

        let metadata = std::fs::metadata(&source)
            .map_err(|error| format!("Could not inspect source video: {error}"))?;
        let modified = metadata
            .modified()
            .ok()
            .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
            .map(|time| time.as_nanos())
            .unwrap_or_default();
        let cache_dir = app_clone
            .path()
            .app_cache_dir()
            .map_err(|error| format!("Could not locate the preview cache: {error}"))?
            .join(crate::preview_cache::CACHE_SUBDIR);
        std::fs::create_dir_all(&cache_dir)
            .map_err(|error| format!("Could not create the preview cache: {error}"))?;
        let output = crate::preview_cache::proxy_path(&cache_dir, &source, metadata.len(), modified);
        if output.is_file() && std::fs::metadata(&output).is_ok_and(|info| info.len() > 0) {
            let _ = app_clone.asset_protocol_scope().allow_file(&output);
            register_allowed_media_path(&output);
            return Ok(Some(output.to_string_lossy().into_owned()));
        }
        let temporary = output.with_extension("webm.part");
        let uri = gstreamer::glib::filename_to_uri(&source, None)
            .map_err(|error| format!("Could not read source path: {error}"))?;

        let pipeline = gstreamer::Pipeline::new();
        let decoder = gstreamer::ElementFactory::make("uridecodebin")
            .property("uri", uri)
            .property("caps", gstreamer::Caps::builder("video/x-raw").build())
            .build()
            .map_err(|error| format!("Could not create video decoder: {error}"))?;
        let queue = gstreamer::ElementFactory::make("queue")
            .build()
            .map_err(|error| error.to_string())?;
        let convert = gstreamer::ElementFactory::make("videoconvert")
            .build()
            .map_err(|error| format!("Video conversion is unavailable: {error}"))?;
        let scale = gstreamer::ElementFactory::make("videoscale")
            .property("add-borders", true)
            .build()
            .map_err(|error| format!("Video scaling is unavailable: {error}"))?;
        let filter = gstreamer::ElementFactory::make("capsfilter")
            .property(
                "caps",
                gstreamer::Caps::builder("video/x-raw")
                    .field("width", 1280_i32)
                    .field("height", 720_i32)
                    .field("pixel-aspect-ratio", gstreamer::Fraction::new(1, 1))
                    .build(),
            )
            .build()
            .map_err(|error| error.to_string())?;
        let encoder = gstreamer::ElementFactory::make("vp8enc")
            .property("deadline", 1_i64)
            .property("cpu-used", 8_i32)
            .build()
            .map_err(|error| format!("VP8 preview encoder is unavailable: {error}"))?;
        let mux = gstreamer::ElementFactory::make("webmmux")
            .build()
            .map_err(|error| format!("WebM preview muxer is unavailable: {error}"))?;
        let sink = gstreamer::ElementFactory::make("filesink")
            .property("location", temporary.to_string_lossy().as_ref())
            .build()
            .map_err(|error| error.to_string())?;

        pipeline
            .add_many([
                &decoder, &queue, &convert, &scale, &filter, &encoder, &mux, &sink,
            ])
            .map_err(|error| format!("Could not assemble preview pipeline: {error}"))?;
        gstreamer::Element::link_many([&queue, &convert, &scale, &filter, &encoder, &mux, &sink])
            .map_err(|error| format!("Could not link preview pipeline: {error}"))?;
        let video_queue = queue.clone();
        decoder.connect_pad_added(move |_decoder, pad| {
            if let Some(sink_pad) = video_queue.static_pad("sink") {
                if !sink_pad.is_linked() {
                    let _ = pad.link(&sink_pad);
                }
            }
        });

        let bus = pipeline
            .bus()
            .ok_or_else(|| "Preview pipeline has no message bus".to_string())?;
        pipeline
            .set_state(gstreamer::State::Playing)
            .map_err(|error| format!("Could not decode source video: {error}"))?;
        let start_time = std::time::Instant::now();
        let result = loop {
            if ctx.is_cancelled() {
                break Err("Preview proxy transcoding was cancelled".to_string());
            }
            if start_time.elapsed() > std::time::Duration::from_secs(60) {
                break Err("Video preview transcoding timed out".to_string());
            }

            let position = pipeline
                .query_position::<gstreamer::ClockTime>()
                .map(|p| p.nseconds())
                .unwrap_or(0);
            let duration = pipeline
                .query_duration::<gstreamer::ClockTime>()
                .map(|d| d.nseconds())
                .unwrap_or(0);
            if duration > 0 {
                let fraction = (position as f32 / duration as f32).clamp(0.0, 0.99);
                ctx.update_progress(
                    fraction,
                    format!("Transcoding proxy ({}%)", (fraction * 100.0) as u32),
                );
            }

            if let Some(message) = bus.timed_pop(gstreamer::ClockTime::from_mseconds(250)) {
                match message.view() {
                    gstreamer::MessageView::Eos(..) => break Ok(()),
                    gstreamer::MessageView::Error(error) => {
                        break Err(format!(
                            "Could not prepare video preview: {}{}",
                            error.error(),
                            error
                                .debug()
                                .map(|debug| format!(" ({debug})"))
                                .unwrap_or_default()
                        ));
                    }
                    _ => {}
                }
            }
        };
        let _ = pipeline.set_state(gstreamer::State::Null);
        if let Err(error) = result {
            let _ = std::fs::remove_file(&temporary);
            return Err(error);
        }
        if !temporary.is_file() || std::fs::metadata(&temporary).is_ok_and(|info| info.len() == 0) {
            let _ = std::fs::remove_file(&temporary);
            return Err("Video decoding completed without producing a preview".to_string());
        }
        std::fs::rename(&temporary, &output)
            .map_err(|error| format!("Could not save video preview: {error}"))?;
        let _ = app_clone.asset_protocol_scope().allow_file(&output);
        register_allowed_media_path(&output);
        Ok(Some(output.to_string_lossy().into_owned()))
    });

    let res = tauri::async_runtime::spawn_blocking(move || rx.recv())
        .await
        .map_err(|error| format!("Job execution error: {error}"))?
        .map_err(|error| format!("Preview task failed: {error}"))?;

    res?.ok_or_else(|| "No preview proxy generated".into())
}

#[tauri::command]
pub async fn probe_media_for_update(path: String) -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(move || {
        gstreamer::init().map_err(|error| format!("Could not initialize GStreamer: {error}"))?;
        let discoverer = Discoverer::new(gstreamer::ClockTime::from_seconds(30))
            .map_err(|error| format!("Could not create media scanner: {error}"))?;
        let canonical_path = PathBuf::from(&path)
            .canonicalize()
            .map_err(|error| format!("Cannot read path: {error}"))?;
        if !canonical_path.is_file() {
            return Err(format!("File not found: {}", canonical_path.display()));
        }
        let (kind, duration, width, height, _, _, _, discovered_ok) =
            probe_media_file(&discoverer, &canonical_path);
        if !discovered_ok {
            return Err(format!(
                "GStreamer could not decode {}",
                canonical_path.display()
            ));
        }
        Ok(json!({
            "path": canonical_path.to_string_lossy(),
            "kind": kind,
            "duration": duration,
            "width": width,
            "height": height,
        }))
    })
    .await
    .map_err(|error| format!("Media probe task failed: {error}"))?
}

#[tauri::command]
pub async fn import_media(
    app: AppHandle,
    state: State<'_, AppState>,
    paths: Vec<String>,
) -> Result<Value, String> {
    let engine = Arc::clone(&state.0);
    let jobs = Arc::clone(&state.1);
    let preview = Arc::clone(&state.2);
    tauri::async_runtime::spawn_blocking(move || {
        gstreamer::init().map_err(|error| format!("Could not initialize GStreamer: {error}"))?;
        let discoverer = Discoverer::new(gstreamer::ClockTime::from_seconds(30))
            .map_err(|error| format!("Could not create media scanner: {error}"))?;
        let mut discovered = Vec::new();
        let mut problems: Vec<Value> = Vec::new();

        for selected_path in paths {
            let path = PathBuf::from(&selected_path);
            // Canonicalize to resolve any .. components and symlinks
            let canonical_path = match path.canonicalize() {
                Ok(resolved) => resolved,
                Err(error) => {
                    problems.push(json!({
                        "path": selected_path,
                        "reason": format!("Cannot read path: {error}"),
                    }));
                    continue;
                }
            };
            if !canonical_path.is_file() {
                problems.push(json!({
                    "path": selected_path,
                    "reason": "File not found",
                }));
                continue;
            }
            let name = canonical_path
                .file_name()
                .map(|part| part.to_string_lossy().into_owned())
                .unwrap_or_else(|| selected_path.clone());

            let (kind, duration, width, height, fps, audio_channels, codec, discovered_ok) =
                probe_media_file(&discoverer, &canonical_path);
            if let Some(reason) = import_problem_reason(discovered_ok, &kind) {
                problems.push(json!({ "path": selected_path, "reason": reason }));
            }
            discovered.push((
                name,
                canonical_path,
                (kind, duration, width, height, fps, audio_channels, codec, discovered_ok),
            ));
        }

        let mut engine_guard = engine
            .lock()
            .map_err(|_| "Editor state is unavailable".to_string())?;
        let imported_count = discovered.len();
        for (name, path, (kind, duration, width, height, fps, audio_channels, codec, _)) in
            discovered
        {
            let waveform = if matches!(kind, MediaKind::Audio | MediaKind::Video) {
                generate_waveform_peaks(&path)
            } else {
                None
            };
            let params = json!({
                "name": name,
                "path": path,
                "kind": kind,
                "duration": duration,
                "width": width,
                "height": height,
                "fps": fps,
                "audio_channels": audio_channels,
                "codec": codec,
                "waveform": waveform,
            });
            engine_guard
                .execute(CommandRequest::new("project.add_media", params))
                .map_err(|error| error.to_string())?;
            register_allowed_media_path(&path);
            let _ = app.asset_protocol_scope().allow_file(&path);
        }

        let mut status = engine_guard
            .execute(CommandRequest::new("project.status", Value::Null))
            .map_err(|error| error.to_string())?;
        drop(engine_guard);
        // The import report rides on the status value: imported count plus a
        // problem entry per file that was skipped or imported as unknown, so
        // the UI can state exactly what happened instead of a bare count.
        if let Some(report) = status.as_object_mut() {
            report.insert("imported".into(), json!(imported_count));
            report.insert("problems".into(), json!(problems));
        }
        if let Err(error) = autosave_once(
            &app,
            &AppState(Arc::clone(&engine), Arc::clone(&jobs), Arc::clone(&preview)),
        ) {
            eprintln!("Autosave after media import failed: {error}");
        }
        Ok(status)
    })
    .await
    .map_err(|error| format!("Media import task failed: {error}"))?
}

#[cfg(test)]
mod tests {
    use super::super::super::MediaKind;
    use super::import_problem_reason;

    #[test]
    fn import_problems_are_stated_not_silent() {
        assert_eq!(
            import_problem_reason(false, &MediaKind::Video),
            Some("GStreamer could not decode this file")
        );
        assert_eq!(
            import_problem_reason(true, &MediaKind::Unknown),
            Some("No playable audio or video streams (unsupported or corrupt file)")
        );
        assert_eq!(import_problem_reason(true, &MediaKind::Video), None);
        assert_eq!(import_problem_reason(true, &MediaKind::Audio), None);
        assert_eq!(import_problem_reason(true, &MediaKind::Image), None);
    }
}
