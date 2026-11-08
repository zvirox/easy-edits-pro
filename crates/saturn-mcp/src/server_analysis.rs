// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Analysis/payload MCP tool methods on [`super::EditorMcp`]: preview, frame
//! sheets, proxies, scope/audio/semantic telemetry, cut continuity and the
//! shot matcher. Split from [`super`] by area so each file stays under the
//! 800-line rule; they journal to nothing here — every tool either reads a
//! snapshot on the blocking pool or routes through the validated command
//! channel.

use super::*;

impl EditorMcp {
    pub(super) async fn timeline_beat_grid_quantize(&self, arguments: &Value) -> Result<Value, String> {
        if matches!(&self.backend, Backend::Bridge(_)) {
            return self
                .bridge_call("timeline_beat_grid_quantize", arguments.clone())
                .await;
        }
        let document = self.headless_document().await?;
        let arguments = arguments.clone();
        let prepared = tokio::task::spawn_blocking(move || {
            saturn_render::beat_grid::prepare(&document, &arguments)
        })
        .await
        .map_err(|e| format!("Beat-grid analysis task failed: {e}"))??;
        let applied = self
            .execute(prepared.command.id, prepared.command.params)
            .await?;
        Ok(json!({"applied": applied, "telemetry": prepared.telemetry}))
    }

    pub(super) async fn edit_match_cut_align(&self, arguments: &Value) -> Result<Value, String> {
        if matches!(&self.backend, Backend::Bridge(_)) {
            return self.bridge_call("edit_match_cut_align", arguments.clone()).await;
        }
        let document = self.headless_document().await?;
        let arguments = arguments.clone();
        let prepared = tokio::task::spawn_blocking(move || {
            saturn_render::match_cut::prepare(&document, &arguments)
        }).await.map_err(|e| format!("Match-cut analysis task failed: {e}"))??;
        let applied = self.execute(prepared.command.id, prepared.command.params).await?;
        Ok(json!({"applied": applied, "telemetry": prepared.telemetry}))
    }

    pub(super) async fn audio_process(&self, arguments: &Value) -> Result<Value, String> {
        if matches!(&self.backend, Backend::Bridge(_)) {
            return self.bridge_call("audio_process", arguments.clone()).await;
        }
        let Backend::Headless(engine) = &self.backend else { unreachable!() };
        let engine = engine.clone();
        let arguments = arguments.clone();
        tokio::task::spawn_blocking(move || {
            let (document, project_path) = {
                let engine = engine.lock().unwrap_or_else(PoisonError::into_inner);
                (engine.document().clone(), engine.current_path().map(ToOwned::to_owned))
            };
            let operation = arguments.get("operation").and_then(Value::as_str).ok_or("operation is required")?;
            let mut edit = saturn_render::audio_edit_driver::prepare(&document, project_path.as_deref(), operation, &arguments)?;
            // Keep the artifact guard with the blocking apply: cancellation of
            // the async waiter must not delete media after the engine commits.
            let applied = engine.lock().unwrap_or_else(PoisonError::into_inner)
                .execute(edit.command.clone()).map_err(|e| e.to_string())?;
            edit.commit();
            Ok(json!({"applied": applied, "telemetry": edit.telemetry}))
        }).await.map_err(|e| format!("Audio processing task failed: {e}"))?
    }

    pub(super) async fn media_preview(&self, arguments: &Value) -> Result<Value, String> {
        let id = arguments
            .get("media_id")
            .and_then(Value::as_u64)
            .ok_or_else(|| "media_preview requires an unsigned media_id".to_owned())?;
        let poster_tick = arguments
            .get("poster_tick")
            .and_then(Value::as_i64)
            .map(saturn_core::Tick);
        if matches!(&self.backend, Backend::Bridge(_)) {
            return self.bridge_call("media_preview", arguments.clone()).await;
        }
        let document = self.headless_document().await?;
        tokio::task::spawn_blocking(move || {
            saturn_render::preview_tools::media_preview_payload(&document, id, poster_tick)
        })
        .await
        .map_err(|error| format!("preview task failed: {error}"))?
    }

    /// `vision_extract_frame_sheet`: tiled, timecode-burned contact sheet of one
    /// media asset for visual inspection. Decode-bound; runs on the blocking
    /// pool. Bridge mode delegates to the desktop app (same shared payload).
    pub(super) async fn vision_extract_frame_sheet(
        &self,
        arguments: &Value,
    ) -> Result<Value, String> {
        if matches!(&self.backend, Backend::Bridge(_)) {
            return self
                .bridge_call("vision_extract_frame_sheet", arguments.clone())
                .await;
        }
        let document = self.headless_document().await?;
        let arguments = arguments.clone();
        tokio::task::spawn_blocking(move || {
            saturn_render::preview_tools::frame_sheet_payload(&document, &arguments.clone())
        })
        .await
        .map_err(|error| format!("contact-sheet task failed: {error}"))?
    }

    /// `render_preview_clip_proxy`: fast low-resolution MP4 snippet of the active
    /// sequence into the isolated previews directory. Bridge mode delegates to
    /// the desktop app.
    pub(super) async fn render_preview_clip_proxy(
        &self,
        arguments: &Value,
    ) -> Result<Value, String> {
        if matches!(&self.backend, Backend::Bridge(_)) {
            return self
                .bridge_call("render_preview_clip_proxy", arguments.clone())
                .await;
        }
        let document = self.headless_document().await?;
        let arguments = arguments.clone();
        tokio::task::spawn_blocking(move || {
            saturn_render::preview_tools::preview_clip_proxy_payload(&document, &arguments.clone())
        })
        .await
        .map_err(|error| format!("proxy render task failed: {error}"))?
    }

    /// `color_read_scopes`: measurable scope telemetry over one frame —
    /// source-frame mode (`media_id` + `tick`) or graded-sequence mode
    /// (`at_tick`, which renders the timeline frame first). Bridge mode
    /// delegates to the desktop app; headless analyzes its own session.
    pub(super) async fn color_read_scopes(&self, arguments: &Value) -> Result<Value, String> {
        if matches!(&self.backend, Backend::Bridge(_)) {
            return self
                .bridge_call("color_read_scopes", arguments.clone())
                .await;
        }
        let document = self.headless_document().await?;
        let arguments = arguments.clone();
        tokio::task::spawn_blocking(move || {
            saturn_render::preview_tools::color_scopes_payload(&document, &arguments.clone())
        })
        .await
        .map_err(|error| format!("scopes task failed: {error}"))?
    }

    /// `color_match_shot_to_reference`: fit a primary-wheels correction from
    /// one clip's source frame against a reference clip's source frame, then
    /// apply it through the validated engine command (both modes). Decode +
    /// fit run on the blocking pool; only the checkpointed command
    /// application touches the engine.
    pub(super) async fn color_match_shot_to_reference(
        &self,
        arguments: &Value,
    ) -> Result<Value, String> {
        if matches!(&self.backend, Backend::Bridge(_)) {
            return self
                .bridge_call("color_match_shot_to_reference", arguments.clone())
                .await;
        }
        let document = self.headless_document().await?;
        let arguments = arguments.clone();
        let payload = tokio::task::spawn_blocking(move || {
            saturn_render::shot_match_driver::match_shot_payload(&document, &arguments)
        })
        .await
        .map_err(|error| format!("shot match task failed: {error}"))??;
        let command = payload
            .get("command")
            .ok_or_else(|| "shot match driver returned no command".to_owned())?;
        let id = command
            .get("id")
            .and_then(Value::as_str)
            .ok_or_else(|| "shot match driver command has no id".to_owned())?
            .to_owned();
        let command_params = command.get("params").cloned().unwrap_or(json!({}));
        let applied = self.execute(id, command_params).await?;
        let telemetry = payload.get("telemetry").cloned().unwrap_or(json!({}));
        Ok(json!({
            "applied": applied,
            "nodeId": applied.get("nodeId").cloned().unwrap_or(Value::Null),
            "telemetry": telemetry,
        }))
    }

    /// `audio_read_telemetry`: measurable audio telemetry over one range —
    /// source-file mode (`media_id` [+ `start_tick`/`end_tick`]) or mix mode
    /// (`start_tick`+`end_tick`, rendering the sequence audio first). Bridge
    /// mode delegates to the desktop app; headless analyzes its own session.
    pub(super) async fn audio_read_telemetry(&self, arguments: &Value) -> Result<Value, String> {
        if matches!(&self.backend, Backend::Bridge(_)) {
            return self
                .bridge_call("audio_read_telemetry", arguments.clone())
                .await;
        }
        let document = self.headless_document().await?;
        let arguments = arguments.clone();
        tokio::task::spawn_blocking(move || {
            saturn_render::preview_tools::audio_telemetry_payload(&document, &arguments.clone())
        })
        .await
        .map_err(|error| format!("audio telemetry task failed: {error}"))?
    }

    /// `media_semantic_query`: SigLIP vector scan over the semantic index
    /// sidecar (perception runs offline; inference happens in a blocking
    /// thread, never while the engine lock is held).
    pub(super) async fn media_semantic_query(&self, arguments: &Value) -> Result<Value, String> {
        if matches!(&self.backend, Backend::Bridge(_)) {
            return self
                .bridge_call("media_semantic_query", arguments.clone())
                .await;
        }
        let document = self.headless_document().await?;
        let project_path = self.headless_project_path().await?;
        let arguments = arguments.clone();
        tokio::task::spawn_blocking(move || {
            saturn_render::preview_tools::semantic_query_payload(
                &document,
                project_path.as_deref(),
                &arguments,
            )
        })
        .await
        .map_err(|error| format!("semantic query task failed: {error}"))?
    }

    /// `edit_evaluate_cut_continuity`: measured continuity report for one
    /// hard cut (frame decode + perception metrics in a blocking thread).
    pub(super) async fn edit_evaluate_cut_continuity(
        &self,
        arguments: &Value,
    ) -> Result<Value, String> {
        if matches!(&self.backend, Backend::Bridge(_)) {
            return self
                .bridge_call("edit_evaluate_cut_continuity", arguments.clone())
                .await;
        }
        let document = self.headless_document().await?;
        let arguments = arguments.clone();
        tokio::task::spawn_blocking(move || {
            saturn_render::preview_tools::cut_continuity_payload(&document, &arguments)
        })
        .await
        .map_err(|error| format!("cut continuity task failed: {error}"))?
    }

    /// The saved project file's path (in-memory documents have none — tools
    /// that need sidecar files report that honestly).
    pub(super) async fn headless_project_path(&self) -> Result<Option<PathBuf>, String> {
        let Backend::Headless(engine) = &self.backend else {
            unreachable!("headless_project_path is only called after the bridge check")
        };
        let engine = engine.clone();
        tokio::task::spawn_blocking(move || {
            let engine = engine.lock().unwrap_or_else(PoisonError::into_inner);
            Ok(engine.current_path().map(PathBuf::from))
        })
        .await
        .map_err(|error| format!("project path task failed: {error}"))?
    }
}
